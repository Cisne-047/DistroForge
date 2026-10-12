/**
 * Bridge WebSocket → TCP (rol "websockify" mínimo, ~50 líneas): permite que el
 * cliente noVNC del navegador hable VNC con QEMU, que solo escucha TCP.
 */
import net from "node:net";
import type { Duplex } from "node:stream";
import type { WebSocket } from "ws";

/** Conecta un socket WebSocket a un servidor TCP. Los errores se ignoran
 *  a propósito: el cierre de cualquiera de los dos extremos cierra el otro. */
export function bridge(ws: WebSocket, tcpPort: number, tcpHost = "127.0.0.1"): void {
  const sock: Duplex = net.connect(tcpPort, tcpHost);
  ws.on("message", (data) => sock.write(data as Buffer));
  sock.on("data", (chunk) => ws.send(chunk));
  const close = () => {
    sock.destroy();
    ws.close();
  };
  ws.on("close", close);
  ws.on("error", close);
  sock.on("close", close);
  sock.on("error", close);
}
