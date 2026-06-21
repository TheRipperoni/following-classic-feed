import { useState, useEffect, useCallback } from 'react';
import axios from 'axios';
import { Users, UserCheck, Activity, RefreshCw, BarChart3, Save, Filter, LogIn, LogOut } from 'lucide-react';
import type { UsageStats, Visitor, UserFeedPreference } from './types.js';

const API_BASE_URL = '/api';
const STORAGE_KEY_TOKEN = 'rsky_session_token';
const STORAGE_KEY_DID = 'rsky_user_did';

// Axios instance with auth header helper
function authAxios(token: string | null) {
  const instance = axios.create({ baseURL: API_BASE_URL });
  if (token) {
    instance.defaults.headers.common['Authorization'] = `Bearer ${token}`;
  }
  return instance;
}

function App() {
  const [stats, setStats] = useState<UsageStats | null>(null);
  const [visitors, setVisitors] = useState<Visitor[]>([]);
  const [loading, setLoading] = useState(true);

  // OAuth / session state
  const [sessionToken, setSessionToken] = useState<string | null>(() => localStorage.getItem(STORAGE_KEY_TOKEN));
  const [userDid, setUserDid] = useState<string | null>(() => localStorage.getItem(STORAGE_KEY_DID));
  const [loginHandle, setLoginHandle] = useState('');
  const [loginLoading, setLoginLoading] = useState(false);
  const [loginError, setLoginError] = useState<string | null>(null);

  // Feed preferences state
  const [feedPreferences, setFeedPreferences] = useState<UserFeedPreference | null>(null);
  const [prefLoading, setPrefLoading] = useState(false);
  const [prefSaving, setPrefSaving] = useState(false);
  const [prefError, setPrefError] = useState<string | null>(null);

  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
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
  }, []);

  // Fetch feed preferences using session token
  const fetchFeedPreferences = useCallback(async () => {
    if (!sessionToken) return;
    setPrefLoading(true);
    setPrefError(null);
    try {
      const client = authAxios(sessionToken);
      const res = await client.get<UserFeedPreference[]>('/user_feed_preference');
      if (res.data && res.data.length > 0 && res.data[0]) {
        setFeedPreferences(res.data[0]);
      } else {
        // No existing config — show defaults
        setFeedPreferences({
          did: userDid || '',
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
      if (err.response?.status === 401) {
        // Token expired — log out
        handleLogout();
        return;
      }
      setPrefError(err.message || 'Failed to fetch preferences');
    } finally {
      setPrefLoading(false);
    }
  }, [sessionToken, userDid]);

  const saveFeedPreferences = async () => {
    if (!feedPreferences || !sessionToken) return;
    setPrefSaving(true);
    setPrefError(null);
    try {
      const client = authAxios(sessionToken);
      await client.put('/user_feed_preference', feedPreferences);
      alert('Feed preferences saved successfully!');
    } catch (err: any) {
      if (err.response?.status === 401) {
        handleLogout();
        return;
      }
      setPrefError('Failed to save preferences: ' + (err.message || 'Unknown error'));
    } finally {
      setPrefSaving(false);
    }
  };

  // --- OAuth Login ---

  const handleLogin = async () => {
    const handle = loginHandle.trim().toLowerCase();
    if (!handle) return;
    setLoginLoading(true);
    setLoginError(null);
    try {
      const res = await axios.get(`${API_BASE_URL}/auth/bluesky/login`, {
        params: { handle }
      });
      const authorizeUrl = res.data.authorize_url;
      if (authorizeUrl) {
        // Store the handle so we can verify after redirect
        sessionStorage.setItem('bsky_login_handle', handle);
        window.location.href = authorizeUrl;
      } else {
        setLoginError('Failed to get authorization URL from server');
      }
    } catch (err: any) {
      setLoginError(err.response?.data?.message || err.message || 'Login failed');
    } finally {
      setLoginLoading(false);
    }
  };

  const handleLogout = () => {
    localStorage.removeItem(STORAGE_KEY_TOKEN);
    localStorage.removeItem(STORAGE_KEY_DID);
    setSessionToken(null);
    setUserDid(null);
    setFeedPreferences(null);
  };

  // On mount: read token/DID from URL (OAuth callback redirect) or localStorage
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const tokenFromUrl = params.get('token');

    if (tokenFromUrl) {
      // We got a session token from the OAuth callback redirect
      localStorage.setItem(STORAGE_KEY_TOKEN, tokenFromUrl);
      setSessionToken(tokenFromUrl);

      // Fetch user DID from /auth/me
      const client = authAxios(tokenFromUrl);
      client.get('/auth/me').then(res => {
        const did = res.data.did;
        localStorage.setItem(STORAGE_KEY_DID, did);
        setUserDid(did);
      }).catch(() => {
        // Fallback: show token-based UI without DID
      });

      // Clean URL (remove ?token=xxx)
      window.history.replaceState({}, document.title, window.location.pathname);
    }
  }, []);

  // When session is set and DID is known, load preferences
  useEffect(() => {
    if (sessionToken && userDid) {
      fetchFeedPreferences();
    }
  }, [sessionToken, userDid, fetchFeedPreferences]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const feedCounts = visitors.reduce((acc: { [key: string]: number }, v) => {
    const feed = v.feed || 'unknown';
    acc[feed] = (acc[feed] || 0) + 1;
    return acc;
  }, {});

  const sortedFeeds = Object.entries(feedCounts).sort(([, a], [, b]) => b - a);
  const maxFeedCount = sortedFeeds.length > 0 ? sortedFeeds[0]![1] : 0;

  const isLoggedIn = !!sessionToken && !!userDid;

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
          <div className="flex items-center gap-3">
            {isLoggedIn && (
              <span className="text-xs text-gray-500 dark:text-gray-400 font-mono truncate max-w-[200px]" title={userDid!}>
                {userDid}
              </span>
            )}
            <button
              onClick={fetchData}
              disabled={loading}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors disabled:opacity-50"
            >
              <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
              Refresh
            </button>
          </div>
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
            <Filter className="w-5 h-5 text-gray-500" />
            Feed Settings
          </h3>

          {!isLoggedIn ? (
            <div>
              <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
                Sign in with Bluesky to configure your feed preferences.
              </p>
              <div className="flex gap-3 items-end">
                <div className="flex-1 max-w-xs">
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Your Bluesky Handle</label>
                  <input
                    type="text"
                    value={loginHandle}
                    onChange={(e) => setLoginHandle(e.target.value)}
                    onKeyDown={(e) => { if (e.key === 'Enter') handleLogin(); }}
                    className="w-full px-3 py-2 bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm"
                    placeholder="user.bsky.social"
                    disabled={loginLoading}
                  />
                </div>
                <button
                  onClick={handleLogin}
                  disabled={loginLoading || !loginHandle.trim()}
                  className="flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors disabled:opacity-50 h-[38px]"
                >
                  <LogIn className="w-4 h-4" />
                  {loginLoading ? 'Redirecting...' : 'Sign In with Bluesky'}
                </button>
              </div>
              {loginError && (
                <div className="mt-3 p-3 bg-red-100 border border-red-400 text-red-700 rounded-lg dark:bg-red-900/30 dark:border-red-800 dark:text-red-400 text-sm">
                  {loginError}
                </div>
              )}
            </div>
          ) : (
            <div>
              <div className="flex items-center justify-between mb-4">
                <p className="text-sm text-gray-500 dark:text-gray-400">
                  Signed in as <span className="font-mono font-medium text-gray-700 dark:text-gray-300">{userDid}</span>
                </p>
                <button
                  onClick={handleLogout}
                  className="flex items-center gap-2 px-3 py-1.5 text-sm bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg transition-colors"
                >
                  <LogOut className="w-3.5 h-3.5" />
                  Sign Out
                </button>
              </div>

              {loginError && (
                <div className="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded-lg dark:bg-red-900/30 dark:border-red-800 dark:text-red-400 text-sm">
                  {loginError}
                </div>
              )}

              {prefError && (
                <div className="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded-lg dark:bg-red-900/30 dark:border-red-800 dark:text-red-400 text-sm">
                  {prefError}
                </div>
              )}

              {prefLoading ? (
                <div className="text-center py-8 text-gray-500">
                  <RefreshCw className="w-6 h-6 animate-spin mx-auto mb-2" />
                  <p className="text-sm">Loading your preferences...</p>
                </div>
              ) : feedPreferences ? (
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
              ) : (
                <div className="text-center py-8 text-gray-500">
                  <p className="text-sm">Unable to load preferences. Try signing out and back in.</p>
                </div>
              )}
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
