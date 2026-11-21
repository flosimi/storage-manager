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

type DiskInfo = {
  name: string;
  mount_point: string;
  file_system: string;
  kind: "ssd" | "hdd" | "unknown";
  total: number;
  available: number;
  removable: boolean;
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

function usageLevel(percent: number): string {
  if (percent > 90) return "critical";
  if (percent > 70) return "warn";
  return "ok";
}

function App() {
  const [ram, setRam] = useState<RamSnapshot | null>(null);
  const [processes, setProcesses] = useState<ProcessMemory[]>([]);
  const [disks, setDisks] = useState<DiskInfo[]>([]);

  async function refreshRam() {
    setRam(await invoke<RamSnapshot>("get_ram"));
  }

  async function refreshProcesses() {
    setProcesses(
      await invoke<ProcessMemory[]>("get_top_processes", { limit: 8 }),
    );
  }

  async function refreshDisks() {
    setDisks(await invoke<DiskInfo[]>("get_disks"));
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

  useEffect(() => {
    refreshDisks();
    const id = setInterval(refreshDisks, 10000);
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
              className={`bar-fill ${usageLevel(ramPercent)}`}
              style={{ width: `${ramPercent}%` }}
            />
          </div>
        </div>
      )}

      <h2>Disks</h2>
      {disks
        .filter((d) => d.total > 0)
        .map((d) => {
          const used = d.total - d.available;
          const percent = (used / d.total) * 100;
          return (
            <div className="disk" key={d.mount_point}>
              <p>
                {d.mount_point} ({d.kind.toUpperCase()}, {d.file_system}):{" "}
                {formatBytes(used)} used of {formatBytes(d.total)} (
                {percent.toFixed(0)}%), {formatBytes(d.available)} free
              </p>
              <div className="bar">
                <div
                  className={`bar-fill ${usageLevel(percent)}`}
                  style={{ width: `${percent}%` }}
                />
              </div>
            </div>
          );
        })}

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