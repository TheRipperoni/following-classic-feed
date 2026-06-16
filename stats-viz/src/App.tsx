import { useState, useEffect } from 'react';
import axios from 'axios';
import { Users, UserCheck, Activity, RefreshCw, BarChart3, Settings, Save, Filter } from 'lucide-react';
import type { UsageStats, Visitor, JanitorConfig, UserFeedPreference } from './types.js';

const API_BASE_URL = '/api'; // Using the proxy configured in nginx or vite

function App() {
  const [stats, setStats] = useState<UsageStats | null>(null);
  const [visitors, setVisitors] = useState<Visitor[]>([]);
  const [janitorConfig, setJanitorConfig] = useState<JanitorConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [configSaving, setConfigSaving] = useState(false);
  const [feedPreferences, setFeedPreferences] = useState<UserFeedPreference | null>(null);
  const [prefDid, setPrefDid] = useState('');
  const [prefLoading, setPrefLoading] = useState(false);
  const [prefSaving, setPrefSaving] = useState(false);
  const [prefError, setPrefError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchData = async () => {
    setLoading(true);
    setError(null);
    try {
      const [statsRes, visitorsRes, janitorRes] = await Promise.all([
        axios.get<UsageStats>(`${API_BASE_URL}/stats`),
        axios.get<Visitor[]>(`${API_BASE_URL}/visitors`),
        axios.get<JanitorConfig>(`${API_BASE_URL}/janitor/config`)
      ]);
      setStats(statsRes.data);
      setVisitors(visitorsRes.data);
      setJanitorConfig(janitorRes.data);
    } catch (err: any) {
      setError(err.message || 'Failed to fetch data');
    } finally {
      setLoading(false);
    }
  };

  const fetchFeedPreferences = async () => {
    const trimmedDid = prefDid.trim();
    if (!trimmedDid) return;
    setPrefLoading(true);
    setPrefError(null);
    try {
      const res = await axios.get<UserFeedPreference[]>(`${API_BASE_URL}/user_feed_preference?did=${encodeURIComponent(trimmedDid)}`);
      if (res.data && res.data.length > 0 && res.data[0]) {
        setFeedPreferences(res.data[0]);
      } else {
        // No existing config — show defaults
        setFeedPreferences({
          did: trimmedDid,
          show_replies: true,
          reply_filter_likes: 0,
          reply_filter_followed_only: false,
          show_reposts: true,
          show_quote_posts: true,
          hide_seen_posts: false,
          hide_no_alt_text: false,
        });
      }
    } catch (err: any) {
      setPrefError(err.message || 'Failed to fetch preferences');
    } finally {
      setPrefLoading(false);
    }
  };

  const saveFeedPreferences = async () => {
    if (!feedPreferences) return;
    setPrefSaving(true);
    setPrefError(null);
    try {
      await axios.put(`${API_BASE_URL}/user_feed_preference`, feedPreferences);
      alert('Feed preferences saved successfully!');
    } catch (err: any) {
      setPrefError('Failed to save preferences: ' + (err.message || 'Unknown error'));
    } finally {
      setPrefSaving(false);
    }
  };

  const saveJanitorConfig = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!janitorConfig) return;
    setConfigSaving(true);
    try {
      await axios.put(`${API_BASE_URL}/janitor/config`, janitorConfig);
      alert('Janitor configuration updated successfully!');
    } catch (err: any) {
      alert('Failed to save configuration: ' + (err.message || 'Unknown error'));
    } finally {
      setConfigSaving(false);
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
              <a href="https://following.ripperoni.com" target="_blank" rel="noopener noreferrer" className="hover:text-blue-600 transition-colors">
                Following Ripperoni Stats
              </a>
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

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
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
          <div className="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 flex items-center gap-4">
            <div className="p-3 bg-purple-100 dark:bg-purple-900/30 rounded-full">
              <Users className="text-purple-600 dark:text-purple-400 w-6 h-6" />
            </div>
            <div>
              <p className="text-sm text-gray-500 dark:text-gray-400 uppercase tracking-wider font-semibold">Weekly Unique</p>
              <h2 className="text-3xl font-bold">{stats?.weeklyUniqueVisitors ?? '--'}</h2>
            </div>
          </div>
        </div>

        <div className="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 mb-8">
          <h3 className="text-lg font-bold flex items-center gap-2 mb-4">
            <Settings className="w-5 h-5 text-gray-500" />
            Janitor Configuration
          </h3>
          <form onSubmit={saveJanitorConfig} className="grid grid-cols-1 md:grid-cols-3 gap-4 items-end">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Cron Schedule</label>
              <input
                type="text"
                value={janitorConfig?.cron_schedule || ''}
                onChange={(e) => setJanitorConfig(prev => prev ? { ...prev, cron_schedule: e.target.value } : null)}
                className="w-full px-3 py-2 bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm"
                placeholder="0 0 0 * * *"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Retention Days</label>
              <input
                type="number"
                value={janitorConfig?.retention_days || 0}
                onChange={(e) => setJanitorConfig(prev => prev ? { ...prev, retention_days: parseInt(e.target.value) || 0 } : null)}
                className="w-full px-3 py-2 bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm"
              />
            </div>
            <button
              type="submit"
              disabled={configSaving || !janitorConfig}
              className="flex items-center justify-center gap-2 px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg transition-colors disabled:opacity-50 h-[38px]"
            >
              <Save className="w-4 h-4" />
              {configSaving ? 'Saving...' : 'Save Settings'}
            </button>
          </form>
          {janitorConfig?.updated_at && (
            <p className="mt-2 text-xs text-gray-500 italic">Last updated: {new Date(janitorConfig.updated_at).toLocaleString()}</p>
          )}
        </div>

        <div className="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 mb-8">
          <h3 className="text-lg font-bold flex items-center gap-2 mb-4">
            <Filter className="w-5 h-5 text-gray-500" />
            Feed Settings
          </h3>
          <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
            Configure how the "Following Classic" feed works for you. Enter your DID to load your current settings.
          </p>
          <div className="flex gap-3 items-end mb-6">
            <div className="flex-1">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Your DID</label>
              <input
                type="text"
                value={prefDid}
                onChange={(e) => setPrefDid(e.target.value)}
                onKeyDown={(e) => { if (e.key === 'Enter') fetchFeedPreferences(); }}
                className="w-full px-3 py-2 bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm font-mono"
                placeholder="did:plc:abc123..."
              />
            </div>
            <button
              onClick={fetchFeedPreferences}
              disabled={prefLoading || !prefDid.trim()}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors disabled:opacity-50 h-[38px]"
            >
              <RefreshCw className={`w-4 h-4 ${prefLoading ? 'animate-spin' : ''}`} />
              {prefLoading ? 'Loading...' : 'Load Settings'}
            </button>
          </div>

          {prefError && (
            <div className="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded-lg dark:bg-red-900/30 dark:border-red-800 dark:text-red-400 text-sm">
              {prefError}
            </div>
          )}

          {feedPreferences && (
            <div className="space-y-5">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <label className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer">
                  <div>
                    <span className="font-medium text-sm">Show Replies</span>
                    <p className="text-xs text-gray-500 dark:text-gray-400">Include reply posts in your feed</p>
                  </div>
                  <input
                    type="checkbox"
                    checked={feedPreferences.show_replies}
                    onChange={(e) => setFeedPreferences({ ...feedPreferences, show_replies: e.target.checked })}
                    className="w-5 h-5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 cursor-pointer"
                  />
                </label>

                <label className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer">
                  <div>
                    <span className="font-medium text-sm">Show Reposts</span>
                    <p className="text-xs text-gray-500 dark:text-gray-400">Include reposts in your feed</p>
                  </div>
                  <input
                    type="checkbox"
                    checked={feedPreferences.show_reposts}
                    onChange={(e) => setFeedPreferences({ ...feedPreferences, show_reposts: e.target.checked })}
                    className="w-5 h-5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 cursor-pointer"
                  />
                </label>

                <label className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer">
                  <div>
                    <span className="font-medium text-sm">Show Quote Posts</span>
                    <p className="text-xs text-gray-500 dark:text-gray-400">Include quote posts in your feed</p>
                  </div>
                  <input
                    type="checkbox"
                    checked={feedPreferences.show_quote_posts}
                    onChange={(e) => setFeedPreferences({ ...feedPreferences, show_quote_posts: e.target.checked })}
                    className="w-5 h-5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 cursor-pointer"
                  />
                </label>

                <label className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer">
                  <div>
                    <span className="font-medium text-sm">Followed-Only Replies</span>
                    <p className="text-xs text-gray-500 dark:text-gray-400">Only show replies from people you follow</p>
                  </div>
                  <input
                    type="checkbox"
                    checked={feedPreferences.reply_filter_followed_only}
                    onChange={(e) => setFeedPreferences({ ...feedPreferences, reply_filter_followed_only: e.target.checked })}
                    className="w-5 h-5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 cursor-pointer"
                  />
                </label>

                <label className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer">
                  <div>
                    <span className="font-medium text-sm">Hide Seen Posts</span>
                    <p className="text-xs text-gray-500 dark:text-gray-400">Don't show posts you've already seen</p>
                  </div>
                  <input
                    type="checkbox"
                    checked={feedPreferences.hide_seen_posts}
                    onChange={(e) => setFeedPreferences({ ...feedPreferences, hide_seen_posts: e.target.checked })}
                    className="w-5 h-5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 cursor-pointer"
                  />
                </label>

                <label className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer">
                  <div>
                    <span className="font-medium text-sm">Hide No-Alt-Text Posts</span>
                    <p className="text-xs text-gray-500 dark:text-gray-400">Hide image posts that lack alt text</p>
                  </div>
                  <input
                    type="checkbox"
                    checked={feedPreferences.hide_no_alt_text}
                    onChange={(e) => setFeedPreferences({ ...feedPreferences, hide_no_alt_text: e.target.checked })}
                    className="w-5 h-5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 cursor-pointer"
                  />
                </label>
              </div>

              <div className="flex flex-col md:flex-row md:items-end gap-4">
                <div className="flex-1 max-w-xs">
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Reply Filter: Minimum Likes</label>
                  <input
                    type="number"
                    min="0"
                    value={feedPreferences.reply_filter_likes}
                    onChange={(e) => setFeedPreferences({ ...feedPreferences, reply_filter_likes: parseInt(e.target.value) || 0 })}
                    className="w-full px-3 py-2 bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm"
                    placeholder="0"
                  />
                  <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                    Only show replies that have at least this many likes (0 = show all)
                  </p>
                </div>

                <button
                  onClick={saveFeedPreferences}
                  disabled={prefSaving}
                  className="flex items-center justify-center gap-2 px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg transition-colors disabled:opacity-50 h-[38px]"
                >
                  <Save className="w-4 h-4" />
                  {prefSaving ? 'Saving...' : 'Save Preferences'}
                </button>
              </div>
            </div>
          )}
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
