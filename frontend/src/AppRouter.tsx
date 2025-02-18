import { BrowserRouter as Router, Route, Routes } from 'react-router-dom';
import Home from './CoinMarketTable';
import CurrencyDetailPage from './CurrencyDetailPage';

const AppRouter = () => {
    return (
        <Router>
            <Routes>
                <Route path="/" element={<Home />} />
                <Route path="/currency/:symbol" element={<CurrencyDetailPage />} />
            </Routes>
        </Router>
    );
};

export default AppRouter;