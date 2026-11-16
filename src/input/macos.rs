//! macOS: consulta o estado de cada tecla (`CGEventSourceKeyState`).
//!
//! Funciona com a janela fora de foco. Como no Windows, vê todos os teclados
//! conectados, não só este. Os códigos de tecla do macOS são convertidos para
//! os do Linux, que é o que a calibração salva.
//!
//! Teclas de multimídia (volume, mudo) não passam por essa API.

use super::InputState;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceKeyState(state_id: i32, key: u16) -> bool;
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

/// Estado do hardware, sem contar teclas simuladas por software.
const HID_SYSTEM_STATE: i32 = 1;

/// Intervalo entre consultas. Curto o bastante para não perder toques rápidos.
const POLL: Duration = Duration::from_millis(8);

/// (código de tecla do macOS, código evdev do Linux).
const KEYS: &[(u16, u16)] = &[
    (0x00, 30),  // A
    (0x01, 31),  // S
    (0x02, 32),  // D
    (0x03, 33),  // F
    (0x04, 35),  // H
    (0x05, 34),  // G
    (0x06, 44),  // Z
    (0x07, 45),  // X
    (0x08, 46),  // C
    (0x09, 47),  // V
    (0x0A, 86),  // \ ao lado do Shift (ISO/ABNT2)
    (0x0B, 48),  // B
    (0x0C, 16),  // Q
    (0x0D, 17),  // W
    (0x0E, 18),  // E
    (0x0F, 19),  // R
    (0x10, 21),  // Y
    (0x11, 20),  // T
    (0x12, 2),   // 1
    (0x13, 3),   // 2
    (0x14, 4),   // 3
    (0x15, 5),   // 4
    (0x16, 7),   // 6
    (0x17, 6),   // 5
    (0x18, 13),  // =
    (0x19, 10),  // 9
    (0x1A, 8),   // 7
    (0x1B, 12),  // -
    (0x1C, 9),   // 8
    (0x1D, 11),  // 0
    (0x1E, 27),  // ]
    (0x1F, 24),  // O
    (0x20, 22),  // U
    (0x21, 26),  // [
    (0x22, 23),  // I
    (0x23, 25),  // P
    (0x24, 28),  // Enter
    (0x25, 38),  // L
    (0x26, 36),  // J
    (0x27, 40),  // '
    (0x28, 37),  // K
    (0x29, 39),  // ;
    (0x2A, 43),  // \
    (0x2B, 51),  // ,
    (0x2C, 53),  // /
    (0x2D, 49),  // N
    (0x2E, 50),  // M
    (0x2F, 52),  // .
    (0x30, 15),  // Tab
    (0x31, 57),  // Espaço
    (0x32, 41),  // `
    (0x33, 14),  // Backspace
    (0x35, 1),   // Esc
    (0x36, 126), // Command direito
    (0x37, 125), // Command esquerdo
    (0x38, 42),  // Shift esquerdo
    (0x39, 58),  // Caps Lock
    (0x3A, 56),  // Option esquerdo
    (0x3B, 29),  // Control esquerdo
    (0x3C, 54),  // Shift direito
    (0x3D, 100), // Option direito (AltGr)
    (0x3E, 97),  // Control direito
    (0x41, 83),  // , do numérico
    (0x43, 55),  // * do numérico
    (0x45, 78),  // + do numérico
    (0x47, 69),  // Num Lock ("Clear" no macOS)
    (0x4B, 98),  // / do numérico
    (0x4C, 96),  // Enter do numérico
    (0x4E, 74),  // - do numérico
    (0x52, 82),  // numérico 0
    (0x53, 79),  // numérico 1
    (0x54, 80),  // numérico 2
    (0x55, 81),  // numérico 3
    (0x56, 75),  // numérico 4
    (0x57, 76),  // numérico 5
    (0x58, 77),  // numérico 6
    (0x59, 71),  // numérico 7
    (0x5B, 72),  // numérico 8
    (0x5C, 73),  // numérico 9
    (0x5E, 89),  // /? do ABNT2
    (0x5F, 121), // . do numérico ABNT2
    (0x60, 63),  // F5
    (0x61, 64),  // F6
    (0x62, 65),  // F7
    (0x63, 61),  // F3
    (0x64, 66),  // F8
    (0x65, 67),  // F9
    (0x67, 87),  // F11
    (0x69, 99),  // PrintScreen (F13 no macOS)
    (0x6B, 70),  // Scroll Lock (F14 no macOS)
    (0x6D, 68),  // F10
    (0x6E, 127), // Menu
    (0x6F, 88),  // F12
    (0x71, 119), // Pause (F15 no macOS)
    (0x72, 110), // Insert ("Help" no macOS)
    (0x73, 102), // Home
    (0x74, 104), // PgUp
    (0x75, 111), // Delete
    (0x76, 62),  // F4
    (0x77, 107), // End
    (0x78, 60),  // F2
    (0x79, 109), // PgDn
    (0x7A, 59),  // F1
    (0x7B, 105), // ←
    (0x7C, 106), // →
    (0x7D, 108), // ↓
    (0x7E, 103), // ↑
];

/// Caps Lock: a API devolve a trava (ligada/desligada), não a tecla. Cada
/// mudança vira um toque curto, com esta duração em consultas.
const CAPS_LOCK: u16 = 0x39;
const CAPS_PULSE: u8 = 12;

const NO_ACCESS: &str = "sem leitura de teclas: libere o app que abriu o keyglow em Ajustes do Sistema > \
                         Privacidade e Segurança > Monitoramento de Entrada e abra o keyglow de novo";

pub fn spawn(state: Arc<Mutex<InputState>>) {
    thread::Builder::new()
        .name("input-poll".into())
        .spawn(move || {
            // SAFETY: funções do CoreGraphics sem ponteiros.
            let allowed = unsafe {
                // Se o sistema exigir a permissão de Monitoramento de Entrada,
                // isto põe o app na lista de Ajustes > Privacidade e Segurança.
                CGPreflightListenEventAccess() || CGRequestListenEventAccess()
            };
            eprintln!("teclas: Monitoramento de Entrada {}", if allowed { "liberado" } else { "não liberado" });
            // Sem a permissão a consulta pode funcionar mesmo assim; o aviso
            // some na primeira tecla lida.
            let mut logged = false;
            {
                let mut st = state.lock().unwrap();
                if allowed {
                    st.devices = 1;
                } else {
                    st.error = Some(NO_ACCESS.into());
                }
            }
            let mut down = [false; KEYS.len()];
            let mut caps_pulse = 0u8;
            loop {
                for (was, &(mac, linux)) in down.iter_mut().zip(KEYS) {
                    // SAFETY: consulta só de leitura, com um código de tecla válido.
                    let now = unsafe { CGEventSourceKeyState(HID_SYSTEM_STATE, mac) };
                    if now == *was {
                        continue;
                    }
                    *was = now;
                    // As travas desse estado duram microssegundos.
                    let mut st = state.lock().unwrap();
                    if !logged {
                        logged = true;
                        eprintln!("teclas: primeira tecla lida");
                        st.devices = 1;
                        st.error = None;
                    }
                    if mac == CAPS_LOCK {
                        st.record(linux, true);
                        caps_pulse = CAPS_PULSE;
                    } else {
                        st.record(linux, now);
                    }
                }
                if caps_pulse > 0 {
                    caps_pulse -= 1;
                    if caps_pulse == 0 {
                        state.lock().unwrap().record(58, false);
                    }
                }
                thread::sleep(POLL);
            }
        })
        .expect("não consegui criar a thread de entrada");
}
