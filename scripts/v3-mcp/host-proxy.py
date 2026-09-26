"""Local fixture-only provider relay; retain counts, never headers or message bodies."""
import http.server,http.client,threading,json,urllib.parse
class Meter:
 def __init__(self,upstream):
  self.upstream=urllib.parse.urlsplit(upstream);self.records=[];self.lock=threading.Lock()
  try:
   import tiktoken
   self.encoding=tiktoken.get_encoding('o200k_base')
  except ImportError:self.encoding=None
  owner=self
  class Handler(http.server.BaseHTTPRequestHandler):
   protocol_version='HTTP/1.1'
   def log_message(self,*args):pass
   def do_POST(self):
    body=self.rfile.read(int(self.headers.get('Content-Length','0')))
    try:
     value=json.loads(body);tools=value.get('tools',[]);results=[]
     for message in value.get('messages',[]):
      if message.get('role')=='tool':results.append(message.get('content',''))
      content=message.get('content',[])
      if isinstance(content,list):results.extend(p for p in content if isinstance(p,dict) and p.get('type')=='tool_result')
     def metric(v):
      text=json.dumps(v,ensure_ascii=False,separators=(',',':'))
      return {'bytes':len(text.encode()),'o200k_reference_tokens':len(owner.encoding.encode(text,disallowed_special=())) if owner.encoding else None}
     record={'request_model':value.get('model'),'tools':metric(tools),'tool_count':len(tools),'visible_tool_results':metric(results),'visible_tool_result_count':len(results)}
     with owner.lock:owner.records.append(record)
    except (ValueError,TypeError,AttributeError):pass
    connection=(http.client.HTTPSConnection if owner.upstream.scheme=='https' else http.client.HTTPConnection)(owner.upstream.netloc,timeout=120)
    headers={k:v for k,v in self.headers.items() if k.lower() not in ['host','connection','content-length','accept-encoding']}
    headers['Host']=owner.upstream.netloc
    try:
     connection.request('POST',self.path,body,headers);response=connection.getresponse();self.send_response(response.status)
     for k,v in response.getheaders():
      if k.lower() not in ['transfer-encoding','connection','content-length']:self.send_header(k,v)
     self.send_header('Connection','close');self.end_headers()
     while data:=response.read1(65536):self.wfile.write(data);self.wfile.flush()
    except (OSError,http.client.HTTPException):pass
    finally:connection.close();self.close_connection=True
  self.server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
  self.thread=threading.Thread(target=self.server.serve_forever,daemon=True);self.thread.start()
  self.base_url='http://127.0.0.1:'+str(self.server.server_port)+self.upstream.path.rstrip('/')
 def close(self):self.server.shutdown();self.server.server_close()
