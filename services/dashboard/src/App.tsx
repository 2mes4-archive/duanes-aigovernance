import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import React from "react";
import ReactDOM from "react-dom/client";

const COLLECTOR_URL = import.meta.env.VITE_COLLECTOR_URL || "http://localhost:9090";

const queryClient = new QueryClient({ refetchInterval: 30000 });

function useOverview() {
  return useQuery({
    queryKey: ["overview"],
    queryFn: async () => {
      const res = await fetch(`${COLLECTOR_URL}/api/v1/stats/overview`);
      return res.json();
    },
  });
}

function useByModel() {
  return useQuery({
    queryKey: ["by-model"],
    queryFn: async () => {
      const res = await fetch(`${COLLECTOR_URL}/api/v1/stats/by-model`);
      return res.json();
    },
  });
}

function useByUser() {
  return useQuery({
    queryKey: ["by-user"],
    queryFn: async () => {
      const res = await fetch(`${COLLECTOR_URL}/api/v1/stats/by-user`);
      return res.json();
    },
  });
}

function useBySource() {
  return useQuery({
    queryKey: ["by-source"],
    queryFn: async () => {
      const res = await fetch(`${COLLECTOR_URL}/api/v1/stats/by-source`);
      return res.json();
    },
  });
}

function MetricCard({ label, value, unit }: { label: string; value: string | number; unit?: string }) {
  return (
    <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
      <p className="text-sm font-medium text-gray-500">{label}</p>
      <p className="mt-2 text-3xl font-bold text-gray-900">
        {value}
        {unit && <span className="ml-1 text-base font-normal text-gray-500">{unit}</span>}
      </p>
    </div>
  );
}

function Dashboard() {
  const overview = useOverview();
  const models = useByModel();
  const users = useByUser();
  const sources = useBySource();

  const data = overview.data || {};

  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white border-b border-gray-200 px-6 py-4">
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          <h1 className="text-xl font-bold text-gray-900">Duanes Governance Dashboard</h1>
          <span className="text-sm text-gray-500">Real-time AI Infrastructure Monitoring</span>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-6 py-8">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          <MetricCard label="Total Requests" value={data.total_requests ?? "—"} />
          <MetricCard label="Total Tokens" value={data.total_tokens ?? "—"} />
          <MetricCard label="Total Cost" value={`$${(data.total_cost ?? 0).toFixed(4)}`} />
          <MetricCard label="Avg Latency" value={Math.round(data.avg_latency_ms ?? 0)} unit="ms" />
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">Cost by Model</h2>
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200">
                  <th className="text-left py-2 text-gray-500">Model</th>
                  <th className="text-right py-2 text-gray-500">Requests</th>
                  <th className="text-right py-2 text-gray-500">Tokens</th>
                  <th className="text-right py-2 text-gray-500">Cost</th>
                  <th className="text-right py-2 text-gray-500">Latency</th>
                </tr>
              </thead>
              <tbody>
                {(models.data?.models || []).map((m: any) => (
                  <tr key={m.model} className="border-b border-gray-100">
                    <td className="py-2 font-mono text-xs">{m.model}</td>
                    <td className="text-right py-2">{m.total_requests}</td>
                    <td className="text-right py-2">{m.total_tokens}</td>
                    <td className="text-right py-2">${m.total_cost.toFixed(4)}</td>
                    <td className="text-right py-2">{Math.round(m.avg_latency_ms)}ms</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">Cost by User</h2>
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200">
                  <th className="text-left py-2 text-gray-500">User</th>
                  <th className="text-right py-2 text-gray-500">Requests</th>
                  <th className="text-right py-2 text-gray-500">Tokens</th>
                  <th className="text-right py-2 text-gray-500">Cost</th>
                </tr>
              </thead>
              <tbody>
                {(users.data?.users || []).slice(0, 20).map((u: any) => (
                  <tr key={u.user_id} className="border-b border-gray-100">
                    <td className="py-2 font-mono text-xs">{u.user_id}</td>
                    <td className="text-right py-2">{u.total_requests}</td>
                    <td className="text-right py-2">{u.total_tokens}</td>
                    <td className="text-right py-2">${u.total_cost.toFixed(4)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="bg-white rounded-xl shadow-sm border border-gray-200 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">Edge Sources</h2>
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200">
                  <th className="text-left py-2 text-gray-500">Source</th>
                  <th className="text-right py-2 text-gray-500">Requests</th>
                  <th className="text-right py-2 text-gray-500">Tokens</th>
                  <th className="text-right py-2 text-gray-500">Cost</th>
                </tr>
              </thead>
              <tbody>
                {(sources.data?.sources || []).map((s: any) => (
                  <tr key={s.source_id} className="border-b border-gray-100">
                    <td className="py-2 font-mono text-xs">{s.source_id}</td>
                    <td className="text-right py-2">{s.total_requests}</td>
                    <td className="text-right py-2">{s.total_tokens}</td>
                    <td className="text-right py-2">${s.total_cost.toFixed(4)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </main>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <QueryClientProvider client={queryClient}>
    <Dashboard />
  </QueryClientProvider>
);
