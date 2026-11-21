import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type RamSnapshot = {
  total: number;
  used: number;
  available: number;
  swap_total: number;
  swap_used: number;
};

type ProcessMemory = { pid: number; name: string; memory_bytes: number };

function formatBytes(bytes: number): string {
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return unit === 0 ? `${bytes} B` : `${value.toFixed(1)} ${units[unit]}`;
}
function ramLevel(percent: number): string{
  if (percent >90) return "critical";
  if (percent >70) return "warn";
  return "ok";
}

function App() {
  const [ram, setRam] = useState<RamSnapshot | null>(null);
  const [processes, setProcesses] = useState<ProcessMemory[]>([]);

  async function refreshRam() {
    setRam(await invoke<RamSnapshot>("get_ram"));
  }

  async function refreshProcesses() {
    setProcesses(
      await invoke<ProcessMemory[]>("get_top_processes", { limit: 8 }),
    );
  }

  useEffect(() => {
    const tick = () => {
      refreshRam();
      refreshProcesses();
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);
  const ramPercent = ram ? (ram.used / ram.total) * 100 : 0;

  return (
    <main className="container">
      <h1>Storage Manager</h1>

      {ram && (
  <div className="ram">
    <p>
      RAM: {formatBytes(ram.used)} used of {formatBytes(ram.total)} (
      {ramPercent.toFixed(0)}%)
    </p>
    <div className="bar">
      <div
        className={`bar-fill ${ramLevel(ramPercent)}`}
        style={{ width: `${ramPercent}%` }}
      />
    </div>
  </div>
)}

      <table>
        <thead>
          <tr>
            <th>Process</th>
            <th>PID</th>
            <th>Memory</th>
          </tr>
        </thead>
        <tbody>
          {processes.map((p) => (
            <tr key={p.pid}>
              <td>{p.name}</td>
              <td>{p.pid}</td>
              <td>{formatBytes(p.memory_bytes)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </main>
  );
}

export default App;