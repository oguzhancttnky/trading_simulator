import { useState, useEffect, useRef } from "react";
import {
  Container,
  Card,
  Group,
  Text,
  Badge,
  Grid,
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
} from "recharts";
import { useParams } from "react-router-dom";

interface SymbolData {
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
      if (isUnmounting.current || wsRef.current?.readyState === WebSocket.OPEN) {
        return;
      }

      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }

      try {
        const ws = new WebSocket(`ws://127.0.0.1:8080/currency/${symbol}`);
        wsRef.current = ws;

        ws.onopen = () => {
          if (!isUnmounting.current) {
            setIsConnected(true);
            setError(null);
          }
        };

        ws.onmessage = (event) => {
          if (!isUnmounting.current) {
            try {
              const parsedData: SymbolData[] = JSON.parse(event.data);
              setData(parsedData);
            } catch (e) {
              console.error("Error parsing message:", e);
            }
          }
        };

        ws.onerror = () => {
          if (!isUnmounting.current) {
            setError("WebSocket error occurred");
            setIsConnected(false);
          }
        };

        ws.onclose = (event) => {
          if (!isUnmounting.current) {
            setIsConnected(false);
            if (!event.wasClean && document.visibilityState !== "hidden") {
              reconnectTimeoutRef.current = window.setTimeout(connectWebSocket, 5000);
            }
          }
        };
      } catch (error) {
        if (!isUnmounting.current) {
          setError("Failed to connect to WebSocket");
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

  const formatPrice = (price: number) => {
    return price.toLocaleString(undefined, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 6,
    });
  };

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
            <Badge variant="dot" color={isConnected ? "green" : "red"} size="lg">
              {isConnected ? "Live" : "Disconnected"}
            </Badge>
          </Group>
          {error && (
            <Text color="red" size="sm" mt="xs">
              {error}
            </Text>
          )}
        </Card.Section>

        <Grid mt="md">
          <Grid.Col span={12}>
            <Card withBorder>
              <Title order={4} mb="md">Price Overview</Title>
              <div style={{ height: 300 }}>
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={data}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="symbol" />
                    <YAxis domain={['auto', 'auto']} />
                    <Tooltip
                      formatter={(value: number) => ['$' + formatPrice(value)]}
                    />
                    <Line
                      type="monotone"
                      dataKey="close_price"
                      stroke="#1c7ed6"
                      name="Close Price"
                    />
                    <Line
                      type="monotone"
                      dataKey="open_price"
                      stroke="#40c057"
                      name="Open Price"
                    />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            </Card>
          </Grid.Col>

          <Grid.Col span={6}>
            <Card withBorder>
              <Title order={4} mb="md">High/Low Prices</Title>
              <div style={{ height: 300 }}>
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart data={data}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="symbol" />
                    <YAxis domain={['auto', 'auto']} />
                    <Tooltip
                      formatter={(value: number) => ['$' + formatPrice(value)]}
                    />
                    <Bar dataKey="high_price" fill="#40c057" name="High Price" />
                    <Bar dataKey="low_price" fill="#fa5252" name="Low Price" />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </Card>
          </Grid.Col>

          <Grid.Col span={6}>
            <Card withBorder>
              <Title order={4} mb="md">Volume</Title>
              <div style={{ height: 300 }}>
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart data={data}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="symbol" />
                    <YAxis domain={['auto', 'auto']} />
                    <Tooltip
                      formatter={(value: number) => [formatVolume(value)]}
                    />
                    <Bar
                      dataKey="quote_volume"
                      fill="#1c7ed6"
                      name="Trading Volume"
                    />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </Card>
          </Grid.Col>
        </Grid>
      </Card>
    </Container>
  );
};

export default CurrencyDetailPage;