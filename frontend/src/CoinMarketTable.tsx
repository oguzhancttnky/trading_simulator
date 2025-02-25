import { useState, useEffect, useRef } from "react";
import {
  Table,
  Pagination,
  Badge,
  Text,
  Group,
  Container,
  Card,
  ActionIcon,
  Tooltip,
} from "@mantine/core";
import { BarChart2 } from "lucide-react";
import { useNavigate } from "react-router-dom";

interface TickerData {
  symbol: string;
  price: number;
  volume: number;
}

interface PaginatedResponse {
  data: TickerData[];
  total: number;
  page: number;
  per_page: number;
}

const CoinMarketTable = () => {
  const [paginatedData, setPaginatedData] = useState<PaginatedResponse>({
    data: [],
    total: 0,
    page: 1,
    per_page: 30,
  });
  const [isConnected, setIsConnected] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);
  const isUnmounting = useRef<boolean>(false);
  const navigate = useNavigate();

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
        const ws = new WebSocket("ws://127.0.0.1:8080");
        wsRef.current = ws;

        ws.onopen = () => {
          if (!isUnmounting.current) {
            console.log("Connected to WebSocket");
            setIsConnected(true);
            setError(null);
            ws.send(
              JSON.stringify({
                page: paginatedData.page,
                per_page: paginatedData.per_page,
              })
            );
          }
        };

        ws.onmessage = (event: MessageEvent) => {
          if (!isUnmounting.current) {
            try {
              const response: PaginatedResponse = JSON.parse(event.data);
              setPaginatedData(response);
            } catch (e) {
              console.error("Error parsing message:", e);
            }
          }
        };

        ws.onerror = (event: Event) => {
          if (!isUnmounting.current) {
            console.error("WebSocket error:", event);
            setError("WebSocket error occurred");
            setIsConnected(false);
          }
        };

        ws.onclose = (event: CloseEvent) => {
          if (!isUnmounting.current) {
            console.log("WebSocket disconnected");
            setIsConnected(false);

            if (
              !event.wasClean &&
              !isUnmounting.current &&
              document.visibilityState !== "hidden"
            ) {
              reconnectTimeoutRef.current = window.setTimeout(
                connectWebSocket,
                5000
              );
            }
          }
        };
      } catch (error) {
        if (!isUnmounting.current) {
          console.error("Failed to create WebSocket connection:", error);
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
  }, []);

  const handlePageChange = (newPage: number) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(
        JSON.stringify({
          page: newPage,
          per_page: paginatedData.per_page,
        })
      );
    }
  };

  const formatPrice = (price: number): string => {
    return price.toLocaleString(undefined, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 6,
    });
  };

  const formatVolume = (volume: number): string => {
    if (volume >= 1_000_000_000) {
      return `$${(volume / 1_000_000_000).toFixed(2)}B`;
    } else if (volume >= 1_000_000) {
      return `$${(volume / 1_000_000).toFixed(2)}M`;
    } else if (volume >= 1_000) {
      return `$${(volume / 1_000).toFixed(2)}K`;
    }
    return `$${volume.toFixed(2)}`;
  };

  const rows = paginatedData.data.map((ticker) => {
    return (
      <Table.Tr key={ticker.symbol} className="hover:bg-gray-50">
        <Table.Td className="font-medium">
          <div>
            <Text size="sm" fw={500}>
              {ticker.symbol}
            </Text>
            <Text size="xs" c="dimmed">
              {ticker.symbol.replace("USDT", "").replace("USDC", "")}
            </Text>
          </div>
        </Table.Td>
        <Table.Td>
          <Text size="sm" fw={500}>
            ${formatPrice(ticker.price)}
          </Text>
        </Table.Td>
        <Table.Td>
          <Text size="sm">{formatVolume(ticker.volume)}</Text>
        </Table.Td>
        <Table.Td>
          <Tooltip label="View Chart">
            <ActionIcon
            onClick={() => navigate(`/currency/${ticker.symbol}`)}
              variant="subtle"
              color="gray"
              size="sm"
              className="opacity-50 hover:opacity-100"
            >
              <BarChart2 size={16} />
            </ActionIcon>
          </Tooltip>
        </Table.Td>
      </Table.Tr>
    );
  });

  const totalPages = Math.ceil(paginatedData.total / paginatedData.per_page);

  return (
    <Container size="lg" className="p-4">
      <Card shadow="sm" p="lg" radius="md">
        <Card.Section className="p-4 border-b">
          <Group justify="center">
            <Text size="lg" fw={500}>
              Cryptocurrency Prices
            </Text>
            <Badge
              variant="dot"
              color={isConnected ? "green" : "red"}
              size="sm"
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

        <Table striped={false} highlightOnHover withTableBorder>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Symbol</Table.Th>
              <Table.Th>Price</Table.Th>
              <Table.Th>Volume</Table.Th>
              <Table.Th></Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {rows.length > 0 ? (
              rows
            ) : (
              <Table.Tr>
                <Table.Td colSpan={5}>
                  <Text c="dimmed" size="sm" ta="center" py="lg">
                    No data available
                  </Text>
                </Table.Td>
              </Table.Tr>
            )}
          </Table.Tbody>
        </Table>

        {totalPages > 1 && (
          <Group justify="center" mt="md">
            <Pagination
              value={paginatedData.page}
              onChange={handlePageChange}
              total={totalPages}
              radius="md"
              withEdges
            />
          </Group>
        )}
      </Card>
    </Container>
  );
};

export default CoinMarketTable;
