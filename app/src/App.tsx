import { useEffect, useState } from "react";
import reactLogo from "./assets/react.svg";
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

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [ram, setRam] = useState<RamSnapshot | null>(null);

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  async function refreshRam() {
    setRam(await invoke<RamSnapshot>("get_ram"));
  }
  const [processes, setProcesses] = useState<ProcessMemory[]>([]);

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

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>

      <div className="row">
        <a href="https://vite.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://react.dev" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <button type="submit">Greet</button>
      </form>
      <p>{greetMsg}</p>

      <button onClick={refreshRam}>Refresh RAM</button>
      {ram && (
        <p>
          RAM: {formatBytes(ram.used)} used of {formatBytes(ram.total)} (
          {((ram.used / ram.total) * 100).toFixed(0)}%)
        </p>
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