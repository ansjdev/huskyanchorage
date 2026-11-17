//! Recebe as cores do SignalRGB por UDP (modo "Do SignalRGB").
//!
//! O plugin `HuskyAnchorage_Controller.js` manda um datagrama a cada
//! `Render()` (~30/s) com as cores que acabou de enviar ao teclado. Quem
//! controla o teclado continua sendo o SignalRGB; o keyglow só desenha.
//!
//! **Protocolo** (um datagrama por quadro, little-endian):
//! ```text
//! [0x4B, 0x47]  "KG"
//! u8            versão (1)
//! u16 LE        número de LEDs
//! [R, G, B] × n cores, na ordem dos LEDs do teclado
//! ```
//! Com 126 LEDs dá 383 bytes: cabe folgado num datagrama.
//!
//! A fila é curta de propósito (`sync_channel`): se o motor estiver ocupado,
//! o quadro novo é descartado em vez de acumular atraso — numa prévia, o
//! quadro mais recente é o único que interessa.

use crate::drivers::Rgb;
use std::net::UdpSocket;
use std::sync::mpsc::{sync_channel, Receiver, TrySendError};
use std::thread;

pub const PORT: u16 = 12721;

const MAGIC: [u8; 2] = [0x4B, 0x47];
const VERSION: u8 = 1;
const HEADER: usize = 5;
const MAX_LEDS: usize = 1024;
/// Cabeçalho + o maior quadro aceito.
const MAX_PACKET: usize = HEADER + MAX_LEDS * 3;
/// Quadros esperando o motor. Pequeno: prévia atrasada não serve para nada.
const QUEUE: usize = 2;

/// Lê um datagrama e devolve as cores, ou `None` se não for um quadro válido.
fn parse(packet: &[u8]) -> Option<Vec<Rgb>> {
    let header = packet.get(..HEADER)?;
    if header[..2] != MAGIC || header[2] != VERSION {
        return None;
    }
    let leds = u16::from_le_bytes([header[3], header[4]]) as usize;
    if leds == 0 || leds > MAX_LEDS {
        return None;
    }
    let body = packet.get(HEADER..HEADER + leds * 3)?;
    Some(body.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

/// Abre a porta e devolve a ponta de leitura dos quadros.
///
/// Erro ao abrir (porta ocupada) não derruba o app: a thread avisa no console
/// e encerra, e a prévia fica no indicador fixo.
pub fn spawn(port: u16) -> Receiver<Vec<Rgb>> {
    let (tx, rx) = sync_channel::<Vec<Rgb>>(QUEUE);

    thread::Builder::new()
        .name("signalrgb-udp".into())
        .spawn(move || {
            let socket = match UdpSocket::bind(("127.0.0.1", port)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("keyglow [signalrgb]: não consegui abrir a porta {port}: {e}");
                    return;
                }
            };
            let mut buf = vec![0u8; MAX_PACKET];
            loop {
                let Ok((len, _from)) = socket.recv_from(&mut buf) else {
                    continue;
                };
                let Some(frame) = parse(&buf[..len]) else {
                    continue;
                };
                match tx.try_send(frame) {
                    // Fila cheia: o motor ainda não consumiu. Joga fora.
                    Ok(()) | Err(TrySendError::Full(_)) => {}
                    // A outra ponta sumiu: ninguém mais quer quadros.
                    Err(TrySendError::Disconnected(_)) => return,
                }
            }
        })
        .expect("não consegui criar a thread do SignalRGB");

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(leds: u16, body: &[u8]) -> Vec<u8> {
        let mut p = vec![MAGIC[0], MAGIC[1], VERSION];
        p.extend_from_slice(&leds.to_le_bytes());
        p.extend_from_slice(body);
        p
    }

    #[test]
    fn reads_the_colors_in_order() {
        let p = packet(2, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(parse(&p), Some(vec![[1, 2, 3], [4, 5, 6]]));
    }

    #[test]
    fn extra_bytes_after_the_frame_are_ignored() {
        let p = packet(1, &[9, 9, 9, 0xFF, 0xFF]);
        assert_eq!(parse(&p), Some(vec![[9, 9, 9]]));
    }

    #[test]
    fn rejects_junk() {
        assert_eq!(parse(&[]), None, "vazio");
        assert_eq!(parse(&packet(2, &[1, 2, 3])), None, "corpo menor que o anunciado");
        assert_eq!(parse(&packet(0, &[])), None, "zero LEDs");
        let mut wrong = packet(1, &[1, 2, 3]);
        wrong[2] = 99;
        assert_eq!(parse(&wrong), None, "outra versão");
        assert_eq!(parse(b"nada a ver"), None, "sem o magic");
    }
}
