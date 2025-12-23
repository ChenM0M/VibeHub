// 协议转换模块：Anthropic <-> OpenAI
// 参考: https://github.com/CassiopeiaCode/b4u2cc

use serde_json::{json, Value};
use std::collections::HashMap;

/// 将 Anthropic Messages API 请求转换为 OpenAI Chat Completions 格式
/// model_mapping: 模型名称映射表，将请求中的模型名映射到目标模型名
pub fn anthropic_to_openai(body: &[u8], model_mapping: &HashMap<String, String>) -> Result<Vec<u8>, String> {
    let anthropic_req: Value = serde_json::from_slice(body)
        .map_err(|e| format!("Failed to parse Anthropic request: {}", e))?;
    
    let mut openai_messages = Vec::new();
    
    // 处理 system 字段
    if let Some(system) = anthropic_req.get("system") {
        if let Some(system_str) = system.as_str() {
            openai_messages.push(json!({
                "role": "system",
                "content": system_str
            }));
        } else if let Some(system_arr) = system.as_array() {
            // Anthropic 的 system 可以是数组格式
            let mut system_content = String::new();
            for item in system_arr {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    system_content.push_str(text);
                    system_content.push('\n');
                }
            }
            if !system_content.is_empty() {
                openai_messages.push(json!({
                    "role": "system",
                    "content": system_content.trim()
                }));
            }
        }
    }
    
    // 转换 messages
    if let Some(messages) = anthropic_req.get("messages").and_then(|m| m.as_array()) {
        for msg in messages {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let openai_role = match role {
                "user" => "user",
                "assistant" => "assistant",
                _ => "user",
            };
            
            // 处理 content
            if let Some(content) = msg.get("content") {
                if let Some(content_str) = content.as_str() {
                    openai_messages.push(json!({
                        "role": openai_role,
                        "content": content_str
                    }));
                } else if let Some(content_arr) = content.as_array() {
                    // 多模态内容块
                    let mut text_parts = Vec::new();
                    for block in content_arr {
                        if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                            match block_type {
                                "text" => {
                                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                        text_parts.push(text.to_string());
                                    }
                                }
                                "tool_result" => {
                                    // 工具结果转换为文本
                                    if let Some(content) = block.get("content") {
                                        if let Some(text) = content.as_str() {
                                            text_parts.push(format!("Tool result: {}", text));
                                        } else if let Some(arr) = content.as_array() {
                                            for item in arr {
                                                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                                    text_parts.push(format!("Tool result: {}", text));
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    if !text_parts.is_empty() {
                        openai_messages.push(json!({
                            "role": openai_role,
                            "content": text_parts.join("\n")
                        }));
                    }
                }
            }
        }
    }
    
    // 构建 OpenAI 请求
    // 获取原始模型名称，并应用模型映射
    let original_model = anthropic_req.get("model")
        .and_then(|m| m.as_str())
        .ok_or("Missing 'model' field in request")?;
    
    // 应用模型映射：如果在映射表中找到，则使用映射后的模型名
    let model = model_mapping.get(original_model)
        .map(|s| s.as_str())
        .unwrap_or(original_model);
    
    let max_tokens = anthropic_req.get("max_tokens")
        .and_then(|m| m.as_u64())
        .unwrap_or(4096);
    
    let temperature = anthropic_req.get("temperature")
        .and_then(|t| t.as_f64())
        .unwrap_or(1.0);
    
    let stream = anthropic_req.get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);
    
    let openai_req = json!({
        "model": model,
        "messages": openai_messages,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "stream": stream
    });
    
    serde_json::to_vec(&openai_req)
        .map_err(|e| format!("Failed to serialize OpenAI request: {}", e))
}

/// 将 OpenAI SSE 事件转换为 Anthropic SSE 格式
/// 输入：OpenAI 的 `data: {...}` 格式
/// 输出：Anthropic 的 `event: xxx\ndata: {...}` 格式
pub fn openai_sse_to_anthropic(openai_line: &str, message_id: &str, model: &str, is_first: bool) -> Vec<String> {
    let mut events = Vec::new();
    
    // 跳过空行和非数据行
    let data = if openai_line.starts_with("data: ") {
        &openai_line[6..]
    } else {
        return events;
    };
    
    // 处理 [DONE]
    if data.trim() == "[DONE]" {
        events.push(format!("event: message_stop\ndata: {{}}"));
        return events;
    }
    
    // 解析 OpenAI 响应
    let openai_resp: Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(_) => return events,
    };
    
    // 如果是第一个事件，发送 message_start
    if is_first {
        events.push(format!(r#"event: message_start
data: {{"type":"message_start","message":{{"id":"{}","type":"message","role":"assistant","content":[],"model":"{}","stop_reason":null,"stop_sequence":null,"usage":{{"input_tokens":0,"output_tokens":0}}}}}}"#, 
            message_id, model));
        
        // 发送 content_block_start
        events.push(format!(r#"event: content_block_start
data: {{"type":"content_block_start","index":0,"content_block":{{"type":"text","text":""}}}}"#));
    }
    
    // 提取 delta content
    if let Some(choices) = openai_resp.get("choices").and_then(|c| c.as_array()) {
        if let Some(choice) = choices.first() {
            // 检查是否完成
            if let Some(finish_reason) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                if finish_reason == "stop" || finish_reason == "end_turn" || finish_reason == "length" {
                    events.push(format!(r#"event: content_block_stop
data: {{"type":"content_block_stop","index":0}}"#));
                    
                    events.push(format!(r#"event: message_delta
data: {{"type":"message_delta","delta":{{"stop_reason":"end_turn","stop_sequence":null}},"usage":{{"output_tokens":0}}}}"#));
                    
                    events.push(format!(r#"event: message_stop
data: {{"type":"message_stop"}}"#));
                    return events;
                }
            }
            
            // 提取文本 delta
            if let Some(delta) = choice.get("delta") {
                if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                    if !content.is_empty() {
                        let escaped = serde_json::to_string(content).unwrap_or_default();
                        // 移除外层引号
                        let escaped = &escaped[1..escaped.len()-1];
                        events.push(format!(r#"event: content_block_delta
data: {{"type":"content_block_delta","index":0,"delta":{{"type":"text_delta","text":"{}"}}}}"#, escaped));
                    }
                }
            }
        }
    }
    
    events
}

/// 将完整的 OpenAI 非流式响应转换为 Anthropic 格式
pub fn openai_response_to_anthropic(openai_body: &[u8], model: &str) -> Result<Vec<u8>, String> {
    let openai_resp: Value = serde_json::from_slice(openai_body)
        .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;
    
    let message_id = format!("msg_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..24].to_string());
    
    let mut content_text = String::new();
    let mut output_tokens = 0u64;
    let mut input_tokens = 0u64;
    
    // 提取 usage
    if let Some(usage) = openai_resp.get("usage") {
        output_tokens = usage.get("completion_tokens").and_then(|c| c.as_u64()).unwrap_or(0);
        input_tokens = usage.get("prompt_tokens").and_then(|p| p.as_u64()).unwrap_or(0);
    }
    
    // 提取 content
    if let Some(choices) = openai_resp.get("choices").and_then(|c| c.as_array()) {
        if let Some(choice) = choices.first() {
            if let Some(message) = choice.get("message") {
                if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                    content_text = content.to_string();
                }
            }
        }
    }
    
    let anthropic_resp = json!({
        "id": message_id,
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": content_text
            }
        ],
        "model": model,
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": input_tokens,
            "output_tokens": output_tokens
        }
    });
    
    serde_json::to_vec(&anthropic_resp)
        .map_err(|e| format!("Failed to serialize Anthropic response: {}", e))
}
