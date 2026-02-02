import { useState, useEffect } from 'react';
import axios from 'axios';
import { Users, UserCheck, Activity, RefreshCw, BarChart3 } from 'lucide-react';
import type { UsageStats, Visitor } from './types.js';

const API_BASE_URL = '/api'; // Using the proxy configured in nginx or vite

function App() {
  const [stats, setStats] = useState<UsageStats | null>(null);
  const [visitors, setVisitors] = useState<Visitor[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchData = async () => {
    setLoading(true);
    setError(null);
    try {
      const [statsRes, visitorsRes] = await Promise.all([
        axios.get<UsageStats>(`${API_BASE_URL}/stats`),
        axios.get<Visitor[]>(`${API_BASE_URL}/visitors`)
      ]);
      setStats(statsRes.data);
      setVisitors(visitorsRes.data);
    } catch (err: any) {
      setError(err.message || 'Failed to fetch data');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
  }, []);

  const feedCounts = visitors.reduce((acc: { [key: string]: number }, v) => {
    const feed = v.feed || 'unknown';
    acc[feed] = (acc[feed] || 0) + 1;
    return acc;
  }, {});

  const sortedFeeds = Object.entries(feedCounts).sort(([, a], [, b]) => b - a);
  const maxFeedCount = sortedFeeds.length > 0 ? sortedFeeds[0]![1] : 0;

  return (
    <div className="min-h-screen bg-gray-50 text-gray-900 dark:bg-gray-900 dark:text-gray-100 p-4 md:p-8">
      <div className="max-w-6xl mx-auto">
        <header className="flex flex-col md:flex-row md:items-center justify-between mb-8 gap-4">
          <div>
            <h1 className="text-3xl font-bold flex items-center gap-2">
              <Activity className="text-blue-500" />
              Following Classic Stats
            </h1>
            <p className="text-gray-500 dark:text-gray-400">Real-time usage and visitor monitoring</p>
          </div>
          <button
            onClick={fetchData}
            disabled={loading}
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors disabled:opacity-50"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </header>

        {error && (
          <div className="mb-8 p-4 bg-red-100 border border-red-400 text-red-700 rounded-lg dark:bg-red-900/30 dark:border-red-800 dark:text-red-400">
            <strong>Error:</strong> {error}. Make sure the backend is running and reachable.
          </div>
        )}

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
          <div className="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 flex items-center gap-4">
            <div className="p-3 bg-blue-100 dark:bg-blue-900/30 rounded-full">
              <Users className="text-blue-600 dark:text-blue-400 w-6 h-6" />
            </div>
            <div>
              <p className="text-sm text-gray-500 dark:text-gray-400 uppercase tracking-wider font-semibold">Total Visits</p>
              <h2 className="text-3xl font-bold">{stats?.totalVisits ?? '--'}</h2>
            </div>
          </div>
          <div className="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 flex items-center gap-4">
            <div className="p-3 bg-green-100 dark:bg-green-900/30 rounded-full">
              <UserCheck className="text-green-600 dark:text-green-400 w-6 h-6" />
            </div>
            <div>
              <p className="text-sm text-gray-500 dark:text-gray-400 uppercase tracking-wider font-semibold">Unique Visitors</p>
              <h2 className="text-3xl font-bold">{stats?.uniqueVisitors ?? '--'}</h2>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          <div className="lg:col-span-2 bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700">
              <h3 className="text-lg font-bold flex items-center gap-2">
                <Users className="w-5 h-5 text-gray-500" />
                Recent Visitors
              </h3>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-left border-collapse">
                <thead>
                  <tr className="bg-gray-50 dark:bg-gray-900/50">
                    <th className="p-4 text-sm font-semibold">DID</th>
                    <th className="p-4 text-sm font-semibold">Feed</th>
                    <th className="p-4 text-sm font-semibold text-right">Time</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100 dark:divide-gray-700">
                  {visitors.length === 0 ? (
                    <tr>
                      <td colSpan={3} className="p-8 text-center text-gray-500">No recent visitors found</td>
                    </tr>
                  ) : (
                    visitors.slice(0, 10).map((v) => (
                      <tr key={v.id} className="hover:bg-gray-50 dark:hover:bg-gray-900/30 transition-colors">
                        <td className="p-4 text-sm font-mono truncate max-w-[200px]" title={v.did}>{v.did}</td>
                        <td className="p-4">
                          <span className="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-xs">
                            {v.feed || 'default'}
                          </span>
                        </td>
                        <td className="p-4 text-sm text-gray-500 text-right">
                          {new Date(v.visited_at).toLocaleString()}
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>

          <div className="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700">
            <h3 className="text-lg font-bold flex items-center gap-2 mb-6">
              <BarChart3 className="w-5 h-5 text-gray-500" />
              Feed Popularity
            </h3>
            <div className="space-y-4">
              {sortedFeeds.length === 0 ? (
                <p className="text-gray-500 text-center py-8">No data available</p>
              ) : (
                sortedFeeds.map(([feed, count]) => (
                  <div key={feed} className="space-y-1">
                    <div className="flex justify-between text-sm">
                      <span className="font-medium">{feed}</span>
                      <span className="text-gray-500">{count}</span>
                    </div>
                    <div className="w-full bg-gray-100 dark:bg-gray-700 rounded-full h-2">
                      <div
                        className="bg-blue-600 h-2 rounded-full transition-all duration-500"
                        style={{ width: `${(count / maxFeedCount) * 100}%` }}
                      ></div>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
