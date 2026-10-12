/**
 * Sesiones de Test Drive: cada sesión arranca una QEMU con la imagen indicada,
 * VNC en localhost y un puente WebSocket para noVNC. TTL máximo y limpieza
 * automática para que el servidor no acumule VMs huérfanas.
 */
import { spawn, type ChildProcess } from "node:child_process";
import { randomUUID } from "node:crypto";
import { WebSocketServer, type WebSocket } from "ws";
import { bridge } from "./vncbridge.js";

export interface DriveSession {
  id: string;
  owner: string;
  qemu: ChildProcess;
  vncPort: number;
  wsPort: number;
  wss: WebSocketServer;
  expires: number;
}

/** Puerto VNC de QEMU: 5900 + display. Reservamos displays altos. */
function vncPortFor(display: number): number {
  return 5900 + display;
}

export class DriveManager {
  private sessions = new Map<string, DriveSession>();
  private nextDisplay = 100;

  constructor(
    private qemuBin: string,
    private ttlMs = 15 * 60 * 1000
  ) {}

  /** ¿Está QEMU disponible en este host? (para poder avisar en la UI) */
  available(): Promise<boolean> {
    return new Promise((resolve) => {
      const p = spawn(this.qemuBin, ["--version"], { stdio: "ignore" });
      p.on("error", () => resolve(false));
      p.on("spawn", () => {
        p.kill();
        resolve(true);
      });
    });
  }

  activeCount(): number {
    this.sweep();
    return this.sessions.size;
  }

  listFor(owner: string): { id: string; expires: number }[] {
    this.sweep();
    return [...this.sessions.values()]
      .filter((s) => s.owner === owner)
      .map((s) => ({ id: s.id, expires: s.expires }));
  }

  /** Arranca una sesión sobre una imagen local del host (ruta absoluta) o una
   *  URL http(s) que QEMU descarga no — las URLs se descargan antes a /tmp. */
  start(owner: string, imagePath: string, memMB = 2048): DriveSession {
    this.sweep();
    const display = this.nextDisplay++;
    const vncPort = vncPortFor(display);
    const qemu = spawn(this.qemuBin, [
      "-m", String(memMB),
      "-display", "none",
      "-vnc", `127.0.0.1:${display}`,
      "-snapshot",
      imagePath,
    ]);
    const wss = new WebSocketServer({ port: 0 });
    const wsPort = (wss.address() as { port: number }).port;
    wss.on("connection", (ws: WebSocket) => bridge(ws, vncPort));
    const session: DriveSession = {
      id: randomUUID(),
      owner,
      qemu,
      vncPort,
      wsPort,
      wss,
      expires: Date.now() + this.ttlMs,
    };
    this.sessions.set(session.id, session);
    qemu.on("exit", () => this.stop(session.id));
    return session;
  }

  get(id: string): DriveSession | undefined {
    return this.sessions.get(id);
  }

  stop(id: string): boolean {
    const s = this.sessions.get(id);
    if (!s) return false;
    this.sessions.delete(id);
    s.qemu.kill("SIGTERM");
    s.wss.close();
    return true;
  }

  /** Mata las sesiones caducadas. */
  private sweep(): void {
    const now = Date.now();
    for (const s of this.sessions.values()) if (s.expires <= now) this.stop(s.id);
  }
}
