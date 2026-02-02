import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { useState, useEffect } from 'react';
import axios from 'axios';
import { Users, UserCheck, Activity, RefreshCw, BarChart3 } from 'lucide-react';
const API_BASE_URL = '/api'; // Using the proxy configured in vite.config.ts
const API_KEY = import.meta.env.VITE_API_KEY || 'test-key';
function App() {
    const [stats, setStats] = useState(null);
    const [visitors, setVisitors] = useState([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const fetchData = async () => {
        setLoading(true);
        setError(null);
        try {
            const [statsRes, visitorsRes] = await Promise.all([
                axios.get(`${API_BASE_URL}/stats`, {
                    headers: { 'Authorization': API_KEY }
                }),
                axios.get(`${API_BASE_URL}/visitors`, {
                    headers: { 'Authorization': API_KEY }
                })
            ]);
            setStats(statsRes.data);
            setVisitors(visitorsRes.data);
        }
        catch (err) {
            setError(err.message || 'Failed to fetch data');
        }
        finally {
            setLoading(false);
        }
    };
    useEffect(() => {
        fetchData();
    }, []);
    const feedCounts = visitors.reduce((acc, v) => {
        const feed = v.feed || 'unknown';
        acc[feed] = (acc[feed] || 0) + 1;
        return acc;
    }, {});
    const sortedFeeds = Object.entries(feedCounts).sort(([, a], [, b]) => b - a);
    const maxFeedCount = sortedFeeds.length > 0 ? sortedFeeds[0][1] : 0;
    return (_jsx("div", { className: "min-h-screen bg-gray-50 text-gray-900 dark:bg-gray-900 dark:text-gray-100 p-4 md:p-8", children: _jsxs("div", { className: "max-w-6xl mx-auto", children: [_jsxs("header", { className: "flex flex-col md:flex-row md:items-center justify-between mb-8 gap-4", children: [_jsxs("div", { children: [_jsxs("h1", { className: "text-3xl font-bold flex items-center gap-2", children: [_jsx(Activity, { className: "text-blue-500" }), "RSky Feedgen Stats"] }), _jsx("p", { className: "text-gray-500 dark:text-gray-400", children: "Real-time usage and visitor monitoring" })] }), _jsxs("button", { onClick: fetchData, disabled: loading, className: "flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors disabled:opacity-50", children: [_jsx(RefreshCw, { className: `w-4 h-4 ${loading ? 'animate-spin' : ''}` }), "Refresh"] })] }), error && (_jsxs("div", { className: "mb-8 p-4 bg-red-100 border border-red-400 text-red-700 rounded-lg dark:bg-red-900/30 dark:border-red-800 dark:text-red-400", children: [_jsx("strong", { children: "Error:" }), " ", error, ". Make sure the backend is running and reachable."] })), _jsxs("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-6 mb-8", children: [_jsxs("div", { className: "bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 flex items-center gap-4", children: [_jsx("div", { className: "p-3 bg-blue-100 dark:bg-blue-900/30 rounded-full", children: _jsx(Users, { className: "text-blue-600 dark:text-blue-400 w-6 h-6" }) }), _jsxs("div", { children: [_jsx("p", { className: "text-sm text-gray-500 dark:text-gray-400 uppercase tracking-wider font-semibold", children: "Total Visits" }), _jsx("h2", { className: "text-3xl font-bold", children: stats?.totalVisits ?? '--' })] })] }), _jsxs("div", { className: "bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 flex items-center gap-4", children: [_jsx("div", { className: "p-3 bg-green-100 dark:bg-green-900/30 rounded-full", children: _jsx(UserCheck, { className: "text-green-600 dark:text-green-400 w-6 h-6" }) }), _jsxs("div", { children: [_jsx("p", { className: "text-sm text-gray-500 dark:text-gray-400 uppercase tracking-wider font-semibold", children: "Unique Visitors" }), _jsx("h2", { className: "text-3xl font-bold", children: stats?.uniqueVisitors ?? '--' })] })] })] }), _jsxs("div", { className: "grid grid-cols-1 lg:grid-cols-3 gap-8", children: [_jsxs("div", { className: "lg:col-span-2 bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden", children: [_jsx("div", { className: "p-6 border-b border-gray-200 dark:border-gray-700", children: _jsxs("h3", { className: "text-lg font-bold flex items-center gap-2", children: [_jsx(Users, { className: "w-5 h-5 text-gray-500" }), "Recent Visitors"] }) }), _jsx("div", { className: "overflow-x-auto", children: _jsxs("table", { className: "w-full text-left border-collapse", children: [_jsx("thead", { children: _jsxs("tr", { className: "bg-gray-50 dark:bg-gray-900/50", children: [_jsx("th", { className: "p-4 text-sm font-semibold", children: "DID" }), _jsx("th", { className: "p-4 text-sm font-semibold", children: "Feed" }), _jsx("th", { className: "p-4 text-sm font-semibold text-right", children: "Time" })] }) }), _jsx("tbody", { className: "divide-y divide-gray-100 dark:divide-gray-700", children: visitors.length === 0 ? (_jsx("tr", { children: _jsx("td", { colSpan: 3, className: "p-8 text-center text-gray-500", children: "No recent visitors found" }) })) : (visitors.slice(0, 10).map((v) => (_jsxs("tr", { className: "hover:bg-gray-50 dark:hover:bg-gray-900/30 transition-colors", children: [_jsx("td", { className: "p-4 text-sm font-mono truncate max-w-[200px]", title: v.did, children: v.did }), _jsx("td", { className: "p-4", children: _jsx("span", { className: "px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-xs", children: v.feed || 'default' }) }), _jsx("td", { className: "p-4 text-sm text-gray-500 text-right", children: new Date(v.visited_at).toLocaleString() })] }, v.id)))) })] }) })] }), _jsxs("div", { className: "bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700", children: [_jsxs("h3", { className: "text-lg font-bold flex items-center gap-2 mb-6", children: [_jsx(BarChart3, { className: "w-5 h-5 text-gray-500" }), "Feed Popularity"] }), _jsx("div", { className: "space-y-4", children: sortedFeeds.length === 0 ? (_jsx("p", { className: "text-gray-500 text-center py-8", children: "No data available" })) : (sortedFeeds.map(([feed, count]) => (_jsxs("div", { className: "space-y-1", children: [_jsxs("div", { className: "flex justify-between text-sm", children: [_jsx("span", { className: "font-medium", children: feed }), _jsx("span", { className: "text-gray-500", children: count })] }), _jsx("div", { className: "w-full bg-gray-100 dark:bg-gray-700 rounded-full h-2", children: _jsx("div", { className: "bg-blue-600 h-2 rounded-full transition-all duration-500", style: { width: `${(count / maxFeedCount) * 100}%` } }) })] }, feed)))) })] })] })] }) }));
}
export default App;
//# sourceMappingURL=App.js.map