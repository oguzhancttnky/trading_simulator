import { useState, useEffect, useRef } from "react";
import {
  Container,
  Card,
  Group,
  Text,
  Badge,
  Stack,
  Title,
} from "@mantine/core";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  LineChart,
  Line,
  Legend,
  ReferenceLine,
} from "recharts";
import { useParams } from "react-router-dom";

interface SymbolData {
  event_time: string;
  symbol: string;
  close_price: number;
  open_price: number;
  high_price: number;
  low_price: number;
  quote_volume: number;
}

const CurrencyDetailPage = () => {
  let { symbol } = useParams();
  if (!symbol) {
    symbol = "BTCUSDT";
  }
  const [data, setData] = useState<SymbolData[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);
  const isUnmounting = useRef(false);

  useEffect(() => {
    isUnmounting.current = false;

    const connectWebSocket = () => {
      if (
        isUnmounting.current ||
        wsRef.current?.readyState === WebSocket.OPEN
      ) {
        return;
      }

      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }

      try {
        // Extract the base symbol without USDT for the WebSocket path
        const baseSymbol = symbol?.replace(/USDT$/, "");
        console.log(
          `Connecting to WebSocket for currency: ${baseSymbol || symbol}`
        );

        // Connect to the WebSocket endpoint
        const ws = new WebSocket(`ws://127.0.0.1:8080/currency/${symbol}`);
        wsRef.current = ws;

        ws.onopen = () => {
          if (!isUnmounting.current) {
            console.log(`WebSocket connection opened for ${symbol}`);
            setIsConnected(true);
            setError(null);
          }
        };

        ws.onmessage = (event) => {
          if (!isUnmounting.current) {
            try {
              const parsedData: SymbolData[] = JSON.parse(event.data);
              if (parsedData.length > 0) {
                // Sort data by event_time in ascending order
                const sortedData = [...parsedData].sort(
                  (a, b) =>
                    new Date(a.event_time).getTime() -
                    new Date(b.event_time).getTime()
                );
                // Process data for chart display
                const processedData = sortedData.map((item) => ({
                  ...item,
                  // Format the event_time for display
                  formattedTime: formatTimeLabel(item.event_time),
                  // Calculate price change for coloring
                  priceChange: item.close_price - item.open_price,
                }));
                setData(processedData);
              } else {
                console.warn("Received empty data array from WebSocket");
              }
            } catch (e) {
              console.error("Error parsing message:", e);
            }
          }
        };

        ws.onerror = (error) => {
          if (!isUnmounting.current) {
            console.error("WebSocket error:", error);
            setError(
              "WebSocket error occurred. Make sure the backend is running."
            );
            setIsConnected(false);
          }
        };

        ws.onclose = (event) => {
          if (!isUnmounting.current) {
            console.log(
              `WebSocket closed for ${symbol}. Code: ${event.code}, Reason: ${event.reason}`
            );
            setIsConnected(false);
            if (!event.wasClean && document.visibilityState !== "hidden") {
              console.log("Attempting to reconnect in 5 seconds...");
              reconnectTimeoutRef.current = window.setTimeout(
                connectWebSocket,
                5000
              );
            }
          }
        };
      } catch (error) {
        if (!isUnmounting.current) {
          console.error("Failed to connect to WebSocket:", error);
          setError(
            `Failed to connect to WebSocket. Make sure the backend is running at 127.0.0.1:8080.`
          );
          setIsConnected(false);
        }
      }
    };

    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        connectWebSocket();
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    connectWebSocket();

    return () => {
      isUnmounting.current = true;
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      if (reconnectTimeoutRef.current !== null) {
        window.clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [symbol]);

  // Format price values with appropriate decimal places
  const formatPrice = (price: number) => {
    return price.toLocaleString(undefined, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 6,
    });
  };

  // Format volume values with K, M, B suffixes
  const formatVolume = (volume: number) => {
    if (volume >= 1_000_000_000) {
      return `$${(volume / 1_000_000_000).toFixed(2)}B`;
    } else if (volume >= 1_000_000) {
      return `$${(volume / 1_000_000).toFixed(2)}M`;
    } else if (volume >= 1_000) {
      return `$${(volume / 1_000).toFixed(2)}K`;
    }
    return `$${volume.toFixed(2)}`;
  };

  // Format timestamp for display
  const formatTimeLabel = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  // Custom tooltip for the OHLC chart
  const PriceTooltip = ({ active, payload }: any) => {
    if (active && payload && payload.length) {
      const data = payload[0].payload;
      return (
        <Card shadow="sm" p="xs" radius="md" withBorder>
          <Stack gap="xs">
            <Text fw={500} size="sm">
              {new Date(data.event_time).toLocaleString()}
            </Text>
            <Text size="xs">Open: ${formatPrice(data.open_price)}</Text>
            <Text size="xs">High: ${formatPrice(data.high_price)}</Text>
            <Text size="xs">Low: ${formatPrice(data.low_price)}</Text>
            <Text size="xs">Close: ${formatPrice(data.close_price)}</Text>
            <Text size="xs">Volume: {formatVolume(data.quote_volume)}</Text>
          </Stack>
        </Card>
      );
    }
    return null;
  };

  return (
    <Container size="xl" className="p-4">
      <Card shadow="sm" p="lg" radius="md">
        <Card.Section className="p-4 border-b">
          <Group justify="space-between">
            <Stack gap="xs">
              <Title order={2}>{symbol}</Title>
              <Text size="sm" c="dimmed">
                {symbol.replace("USDT", "")} Price Chart
              </Text>
            </Stack>
            <Badge
              variant="dot"
              color={isConnected ? "green" : "red"}
              size="lg"
            >
              {isConnected ? "Live" : "Disconnected"}
            </Badge>
          </Group>
          {error && (
            <Text color="red" size="sm" mt="xs">
              {error}
            </Text>
          )}
        </Card.Section>
      </Card>

      <Card withBorder>
        <Title order={4} mb="md">
          Price Chart
        </Title>
        <div style={{ height: 400 }}>
          <ResponsiveContainer width="100%" height="100%">
            <LineChart
              data={data}
              margin={{ top: 20, right: 30, left: 20, bottom: 70 }}
            >
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis
                dataKey="formattedTime"
                angle={-45}
                textAnchor="end"
                height={70}
                tick={{ fontSize: 12 }}
              />
              <YAxis
                domain={["auto", "auto"]}
                tickFormatter={(value) => formatPrice(value)}
              />
              <Tooltip content={<PriceTooltip />} />
              <Legend />
              <ReferenceLine y={0} stroke="#666" />
              <Line
                type="monotone"
                dataKey="close_price"
                stroke="#1c7ed6"
                name="Close Price"
                dot={false}
              />
              <Line
                type="monotone"
                dataKey="open_price"
                stroke="#40c057"
                name="Open Price"
                dot={false}
                strokeDasharray="5 5"
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </Card>
    </Container>
  );
};

export default CurrencyDetailPage;
