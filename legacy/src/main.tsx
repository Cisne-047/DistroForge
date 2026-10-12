import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

class Boundary extends React.Component<
  { children: React.ReactNode },
  { err: Error | null }
> {
  state = { err: null as Error | null };

  static getDerivedStateFromError(err: Error) {
    return { err };
  }

  render() {
    if (this.state.err) {
      return (
        <div style={{ padding: 40, color: "#e06c5f", fontFamily: "ui-monospace, monospace" }}>
          <h2>Error inesperado</h2>
          <pre>{this.state.err.message}</pre>
          <p>
            <a
              href=""
              onClick={(e) => {
                e.preventDefault();
                try {
                  localStorage.removeItem("distroforge-config");
                } catch {
                  // nada que limpiar
                }
                window.location.reload();
              }}
            >
              Reiniciar DistroForge
            </a>
          </p>
        </div>
      );
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Boundary>
      <App />
    </Boundary>
  </React.StrictMode>
);
