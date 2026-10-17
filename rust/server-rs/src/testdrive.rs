//! Sesiones de Test Drive: cada sesión arranca una QEMU con la imagen indicada,
//! VNC en localhost y un puente WebSocket para noVNC (ver `vncbridge`).
//! TTL máximo y limpieza para no acumular VMs huérfanas.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::process::{Child, Command};

/// Puerto VNC de QEMU: 5900 + display. Reservamos displays altos.
fn vnc_port_for(display: u32) -> u16 {
    (5900 + display) as u16
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

type ChildCell = Arc<Mutex<Option<Child>>>;

pub struct DriveSession {
    pub id: String,
    pub owner: String,
    pub vnc_port: u16,
    child: ChildCell,
    pub expires: i64,
}

impl DriveSession {
    /// Mata QEMU sin bloquear (kill_on_drop completa la limpieza).
    fn stop(&self) {
        if let Some(mut child) = self.child.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = child.start_kill();
        }
    }
}

struct Inner {
    qemu_bin: String,
    sessions: Mutex<HashMap<String, Arc<DriveSession>>>,
    next_display: AtomicU32,
    ttl_ms: i64,
}

/// Gestor de sesiones de Test Drive (equivalente a `DriveManager` en TS).
#[derive(Clone)]
pub struct DriveManager(Arc<Inner>);

impl DriveManager {
    pub fn new(qemu_bin: &str, ttl: Duration) -> Self {
        Self(Arc::new(Inner {
            qemu_bin: qemu_bin.to_string(),
            sessions: Mutex::new(HashMap::new()),
            next_display: AtomicU32::new(100),
            ttl_ms: ttl.as_millis() as i64,
        }))
    }

    /// ¿Está QEMU disponible en este host? (para poder avisar en la UI)
    pub async fn available(&self) -> bool {
        match Command::new(&self.0.qemu_bin)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                let _ = child.kill().await;
                true
            }
            Err(_) => false,
        }
    }

    pub fn active_count(&self) -> usize {
        self.sweep();
        self.0
            .sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    pub fn list_for(&self, owner: &str) -> Vec<(String, i64)> {
        self.sweep();
        self.0
            .sessions
            .lock()
            .expect("lock sessions")
            .values()
            .filter(|s| s.owner == owner)
            .map(|s| (s.id.clone(), s.expires))
            .collect()
    }

    /// Arranca una sesión sobre una imagen local del host (ruta absoluta).
    /// Devuelve `Err` si QEMU no puede lanzarse.
    pub async fn start(
        &self,
        owner: &str,
        image_path: &str,
        mem_mb: u32,
    ) -> Result<Arc<DriveSession>, String> {
        self.sweep();
        let display = self.0.next_display.fetch_add(1, Ordering::SeqCst);
        let vnc_port = vnc_port_for(display);
        let child = Command::new(&self.0.qemu_bin)
            .arg("-m")
            .arg(mem_mb.to_string())
            .arg("-display")
            .arg("none")
            .arg("-vnc")
            .arg(format!("127.0.0.1:{display}"))
            .arg("-snapshot")
            .arg(image_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("no se pudo lanzar QEMU: {e}"))?;
        let child_cell: ChildCell = Arc::new(Mutex::new(Some(child)));
        let session = Arc::new(DriveSession {
            id: uuid::Uuid::new_v4().to_string(),
            owner: owner.to_string(),
            vnc_port,
            child: child_cell.clone(),
            expires: now_ms() + self.0.ttl_ms,
        });
        self.0
            .sessions
            .lock()
            .expect("lock sessions")
            .insert(session.id.clone(), session.clone());
        // al morir QEMU se elimina la sesión
        let manager = self.clone();
        let id = session.id.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let mut guard = child_cell.lock().unwrap_or_else(|e| e.into_inner());
                match guard.as_mut() {
                    None => break, // parada manual: ya se borró la sesión
                    Some(child) => match child.try_wait() {
                        Ok(Some(_)) | Err(_) => {
                            guard.take();
                            drop(guard);
                            manager
                                .0
                                .sessions
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .remove(&id);
                            break;
                        }
                        Ok(None) => {}
                    },
                }
            }
        });
        Ok(session)
    }

    pub fn get(&self, id: &str) -> Option<Arc<DriveSession>> {
        self.0
            .sessions
            .lock()
            .expect("lock sessions")
            .get(id)
            .cloned()
    }

    pub fn stop(&self, id: &str) -> bool {
        let Some(session) = self
            .0
            .sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id)
        else {
            return false;
        };
        session.stop();
        true
    }

    /// Mata las sesiones caducadas.
    pub fn sweep(&self) {
        let now = now_ms();
        let expired: Vec<String> = self
            .0
            .sessions
            .lock()
            .expect("lock sessions")
            .iter()
            .filter(|(_, s)| s.expires <= now)
            .map(|(id, _)| id.clone())
            .collect();
        for id in expired {
            self.stop(&id);
        }
    }
}

/// `image_path` absoluto igual que `path.isAbsolute` de Node: en Windows
/// "/tmp/x" cuenta como absoluto (tiene raíz), igual que hace node:path.
pub fn is_absolute_path(p: &str) -> bool {
    let path = Path::new(p);
    path.is_absolute() || path.has_root()
}
