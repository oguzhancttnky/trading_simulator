import { MantineProvider } from '@mantine/core';
import "@mantine/core/styles.css";
import AppRouter from './AppRouter';

function App() {
  return (
    <MantineProvider>
      <AppRouter />
    </MantineProvider>
  );
}

export default App;