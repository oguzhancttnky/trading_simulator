import { MantineProvider } from '@mantine/core';
import CoinMarketTable from './CoinMarketTable';
import "@mantine/core/styles.css";

function App() {
  return (
    <MantineProvider>
      <CoinMarketTable />
    </MantineProvider>
  );
}

export default App;