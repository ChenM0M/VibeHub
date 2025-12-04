import { useTranslation } from 'react-i18next';
import {
    Area,
    AreaChart,
    CartesianGrid,
    ResponsiveContainer,
    Tooltip,
    XAxis,
    YAxis,
} from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

interface ChartData {
    timestamp: number; // Unix timestamp
    requests: number;
    input_tokens: number;
    output_tokens: number;
}

interface RequestChartProps {
    data: ChartData[];
}

export function RequestChart({ data }: RequestChartProps) {
    const { t } = useTranslation();

    // Format data for chart
    const chartData = data.map(d => ({
        ...d,
        time: new Date(d.timestamp * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
    }));

    return (
        <Card className="min-h-[400px]">
            <CardHeader>
                <CardTitle>{t('gateway.stats.hourlyActivity')}</CardTitle>
            </CardHeader>
            <CardContent className="pl-2">
                <div className="h-[300px] w-full">
                    <ResponsiveContainer width="100%" height="100%">
                        <AreaChart data={chartData}>
                            <defs>
                                <linearGradient id="colorRequests" x1="0" y1="0" x2="0" y2="1">
                                    <stop offset="5%" stopColor="#8884d8" stopOpacity={0.8} />
                                    <stop offset="95%" stopColor="#8884d8" stopOpacity={0} />
                                </linearGradient>
                                <linearGradient id="colorTokens" x1="0" y1="0" x2="0" y2="1">
                                    <stop offset="5%" stopColor="#82ca9d" stopOpacity={0.8} />
                                    <stop offset="95%" stopColor="#82ca9d" stopOpacity={0} />
                                </linearGradient>
                            </defs>
                            <XAxis
                                dataKey="time"
                                stroke="#888888"
                                fontSize={12}
                                tickLine={false}
                                axisLine={false}
                            />
                            <YAxis
                                stroke="#888888"
                                fontSize={12}
                                tickLine={false}
                                axisLine={false}
                                tickFormatter={(value) => `${value}`}
                            />
                            <CartesianGrid strokeDasharray="3 3" className="stroke-muted" vertical={false} />
                            <Tooltip
                                contentStyle={{ backgroundColor: 'hsl(var(--popover))', borderColor: 'hsl(var(--border))', borderRadius: 'var(--radius)' }}
                                itemStyle={{ color: 'hsl(var(--popover-foreground))' }}
                            />
                            <Area
                                type="monotone"
                                dataKey="requests"
                                stroke="#8884d8"
                                fillOpacity={1}
                                fill="url(#colorRequests)"
                                name={t('common.requests')}
                            />
                            <Area
                                type="monotone"
                                dataKey="input_tokens"
                                stroke="#82ca9d"
                                fillOpacity={1}
                                fill="url(#colorTokens)"
                                name={t('gateway.input') + ' Tokens'}
                            />
                        </AreaChart>
                    </ResponsiveContainer>
                </div>
            </CardContent>
        </Card>
    );
}
