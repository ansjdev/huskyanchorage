/**
 * Husky Anchorage HTG200 — Plugin SignalRGB
 *
 * Protocolo: EVision V2 (mesmo que o OpenRGB documenta)
 * Interface HID de controle: usage_page 0xFF1C
 * VID:PID 320F:5016
 *
 * Fonte de referência: keyglow (github.com/filipesampay/keyglow)
 * Layout: ABNT2, calibrado em 08/nov/2026 (default_layout.rs do keyglow)
 */

import * as udpModule from "@SignalRGB/udp";

// ─── Metadados do dispositivo ─────────────────────────────────────────────────

export function Name()          { return "Husky Anchorage HTG200"; }
export function VendorId()      { return 0x320F; }
export function ProductId()     { return [0x5016]; }
export function Publisher()     { return "Community"; }
export function Documentation() { return ""; }
export function Size()          { return [24, 6]; }
export function DeviceType()    { return "keyboard"; }
export function ImageUrl()      { return "https://assets.signalrgb.com/devices/default/keyboards/full-size-keyboard-render.png"; }
export function ConflictingProcesses() { return ["TECLADO MECANICO GAMER.exe"]; }

/**
 * Valida a interface de controle EVision V2.
 * O firmware expõe usage_page 0xFF1C na interface de controle.
 * O teclado também expõe uma interface HID padrão (usage_page 0x0001) —
 * essa NÃO é a correta; só a 0xFF1C aceita os comandos de luz.
 */
export function Validate(endpoint) {
    return endpoint.usage_page === 0xFF1C;
}

/* global
shutdownColor:readonly
LightingMode:readonly
forcedColor:readonly
*/
export function ControllableParameters() {
    return [
        {
            property: "shutdownColor", group: "lighting",
            label: "Cor de Desligamento",
            description: "Cor aplicada ao desligar o sistema ou o SignalRGB",
            type: "color", default: "#000000"
        },
        {
            property: "LightingMode", group: "lighting",
            label: "Modo de Iluminação",
            description: "Canvas = efeito ativo do SignalRGB; Forçado = cor fixa",
            type: "combobox", values: ["Canvas", "Forçado"], default: "Canvas"
        },
        {
            property: "forcedColor", group: "lighting",
            label: "Cor Forçada",
            description: "Usada quando o modo Forçado está ativo",
            type: "color", default: "#009bde"
        },
    ];
}

// ─── Mapa de LEDs: [índice_no_frame, nome, x, y] ─────────────────────────────
//
// O firmware usa uma matriz linear de 6×21 = 126 posições (linha por linha).
// LED de índice i ocupa bytes i*3, i*3+1, i*3+2 no frame de cores.
//
// Coordenadas [x, y] mapeiam o canvas do SignalRGB (Size = [24, 6]):
//   x: 0–23 (colunas), y: 0–5 (linhas)
//
// Fonte dos índices: ABNT2_320F_5016 em keyglow/src/default_layout.rs
//   Chave evdev → índice LED → posição visual no teclado

const LEDS = [
    // ── Linha 0 · Função ─────────────────────────────────────────────────────
    [ 0,  "Esc",           0,  0],
    [ 1,  "F1",            2,  0],  [ 2, "F2",    3,  0],  [ 3, "F3",    4,  0],  [ 4, "F4",    5,  0],
    [ 5,  "F5",            7,  0],  [ 6, "F6",    8,  0],  [ 7, "F7",    9,  0],  [ 8, "F8",   10,  0],
    [ 9,  "F9",           12,  0],  [10, "F10",  13,  0],  [11, "F11",  14,  0],  [12, "F12",  15,  0],
    [14,  "Print Screen",  17,  0],
    [17,  "Scroll Lock",   18,  0],
    [16,  "Pause",         19,  0],

    // ── Linha 1 · Números + topo do numpad ───────────────────────────────────
    [21,  "~",              0,  1],
    [22,  "1",              1,  1],  [23, "2",     2,  1],  [24, "3",     3,  1],  [25, "4",     4,  1],
    [26,  "5",              5,  1],  [27, "6",     6,  1],  [28, "7",     7,  1],  [29, "8",     8,  1],
    [30,  "9",              9,  1],  [31, "0",    10,  1],  [32, "-_",   11,  1],  [33, "=+",   12,  1],
    [34,  "Backspace",     14,  1],
    [35,  "Insert",        17,  1],  [36, "Home",  18,  1],  [37, "Page Up", 19, 1],
    [55,  "Num Lock",      20,  1],  [39, "Num /", 21,  1],  [40, "Num *", 22,  1],  [41, "Num -", 23, 1],

    // ── Linha 2 · QWERTY + navegação + numpad superior ───────────────────────
    [42,  "Tab",            0,  2],
    [43,  "Q",              2,  2],  [44, "W",     3,  2],  [45, "E",     4,  2],  [46, "R",     5,  2],
    [47,  "T",              6,  2],  [48, "Y",     7,  2],  [49, "U",     8,  2],  [50, "I",     9,  2],
    [51,  "O",             10,  2],  [52, "P",    11,  2],  [53, "[{",   12,  2],  [54, "]}",   13,  2],
    [56,  "Delete",        17,  2],  [57, "End",   18,  2],  [58, "Page Down", 19, 2],
    [59,  "Num 7",         20,  2],  [60, "Num 8", 21,  2],  [61, "Num 9", 22,  2],
    [62,  "Num +",         23,  2],  // tecla alta — LED no topo

    // ── Linha 3 · Home row + numpad meio ─────────────────────────────────────
    [63,  "Caps Lock",      0,  3],
    [64,  "A",              2,  3],  [65, "S",     3,  3],  [66, "D",     4,  3],  [67, "F",     5,  3],
    [68,  "G",              6,  3],  [69, "H",     7,  3],  [70, "J",     8,  3],  [71, "K",     9,  3],
    [72,  "L",             10,  3],  [73, ";:",   11,  3],  [74, "\"'",  12,  3],  [75, "\\|",  13,  3],
    [76,  "Enter",         15,  3],
    [80,  "Num 4",         20,  3],  [81, "Num 5", 21,  3],  [82, "Num 6", 22,  3],
    [83,  "Num ,",         23,  3],  // KEY_KPCOMMA — específico ABNT2

    // ── Linha 4 · Shift + seta cima + numpad inferior ────────────────────────
    [84,  "Left Shift",     0,  4],
    [85,  "ISO <",          2,  4],  // tecla extra ABNT2 (KEY_102ND)
    [86,  "Z",              3,  4],  [87, "X",     4,  4],  [88, "C",     5,  4],  [89, "V",     6,  4],
    [90,  "B",              7,  4],  [91, "N",     8,  4],  [92, "M",     9,  4],
    [93,  ",<",            10,  4],  [94, ".>",   11,  4],  [95, "/?",   12,  4],
    [96,  "Ç",             13,  4],  // KEY_RO — ABNT2
    [97,  "Right Shift",   15,  4],
    [99,  "Up",            18,  4],
    [101, "Num 1",         20,  4],  [102, "Num 2", 21, 4],  [103, "Num 3", 22, 4],
    [104, "Num Enter",     23,  4],  // tecla alta — LED na base

    // ── Linha 5 · Barra de espaço + setas ────────────────────────────────────
    [105, "Left Ctrl",      0,  5],
    [106, "Left Win",       2,  5],
    [107, "Left Alt",       3,  5],
    [108, "Space",          8,  5],
    [116, "Right Alt",     12,  5],
    [111, "Menu",          13,  5],
    [113, "Right Ctrl",    14,  5],
    [119, "Left",          17,  5],
    [120, "Down",          18,  5],
    [121, "Right",         19,  5],
    [123, "Num 0",         20,  5],
    [124, "Num .",         22,  5],
];

const TOTAL_LEDS = 126; // 6 × 21 — frame de 378 bytes

// ─── Protocolo EVision V2 ─────────────────────────────────────────────────────
//
// Formato do pacote (64 bytes):
//   [0x04, chk_lo, chk_hi, cmd, tamanho, off_lo, off_hi, status=0, dados...]
//   checksum = soma u16 dos bytes 3..63 (inclusive)
//
// Referência: keyglow/src/drivers/evision.rs

const REPORT_ID   = 0x04;
const MAX_PAYLOAD = 56;    // 64 - 8 bytes de cabeçalho

const CMD = {
    BEGIN:        0x01,
    END:          0x02,
    WRITE_CONFIG: 0x06,
    DIRECT:       0x12,
};

const MODE_CUSTOM = 0x13; // desativa efeito de fábrica → modo direto fica estável

/**
 * Monta um pacote EVision de 64 bytes com checksum.
 */
function evisionPacket(cmd, offset, data) {
    const buf = new Array(64).fill(0);
    buf[0] = REPORT_ID;
    buf[3] = cmd;
    buf[4] = data.length;
    buf[5] = offset & 0xFF;
    buf[6] = (offset >> 8) & 0xFF;
    // buf[7] = 0x00 (status — só relevante na resposta)
    for (let i = 0; i < data.length; i++) buf[8 + i] = data[i];

    let sum = 0;
    for (let i = 3; i < 64; i++) sum = (sum + buf[i]) & 0xFFFF;
    buf[1] = sum & 0xFF;
    buf[2] = (sum >> 8) & 0xFF;
    return buf;
}

/**
 * Envia um comando EVision, quebrando em chunks de MAX_PAYLOAD se necessário.
 */
function evisionSend(cmd, offset, data) {
    if (data.length === 0) {
        device.write(evisionPacket(cmd, offset, []), 64);
        return;
    }
    for (let i = 0; i < data.length; i += MAX_PAYLOAD) {
        const chunk = data.slice(i, i + MAX_PAYLOAD);
        device.write(evisionPacket(cmd, offset + i, chunk), 64);
    }
}

/**
 * Coloca o firmware no modo Custom (0x13) — sem efeito de fábrica rodando.
 * No modo direto, o teclado exibe apenas as cores que o software envia.
 * Se o app parar de enviar frames por ~500 ms, o teclado apaga os LEDs
 * até o próximo frame chegar.
 *
 * Sequência: BEGIN → WRITE_CONFIG (perfil 0, offset 0x01) → END
 * Payload do WRITE_CONFIG: [mode, brightness, speed, direction, random, R, G, B]
 */
function enterDirectMode() {
    evisionSend(CMD.BEGIN, 0, []);
    evisionSend(CMD.WRITE_CONFIG, 0x01, [
        MODE_CUSTOM,  // mode = 0x13 (custom/direct)
        0x04,         // brightness = 4 (máximo)
        0x03,         // speed (irrelevante no modo direto)
        0x00,         // direction
        0x00,         // random = false
        0x00, 0x00, 0x00, // R, G, B (irrelevante no modo direto)
    ]);
    evisionSend(CMD.END, 0, []);
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function hexToRgb(hex) {
    const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)];
}

// ─── Funções exportadas ───────────────────────────────────────────────────────

export function Initialize() {
    device.setName("Husky Anchorage HTG200");
    device.setSize([24, 6]);
    device.setControllableLeds(
        LEDS.map(([, name])    => name),
        LEDS.map(([,, x, y])   => [x, y])
    );
    device.setImageFromUrl(ImageUrl());
    enterDirectMode();
    keyglowOpen();
}

export function Render() {
    sendFrame(null);
}

export function Shutdown(SystemSuspending) {
    sendFrame(SystemSuspending ? "#000000" : shutdownColor);
    keyglowClose();
}

/**
 * Monta e envia um frame completo de 126 LEDs (378 bytes).
 *
 * @param {string|null} overrideHex  Cor fixa em hex, ou null para canvas.
 */
function sendFrame(overrideHex) {
    // Frame linear: posição i ocupa bytes i*3, i*3+1, i*3+2
    const frame = new Array(TOTAL_LEDS * 3).fill(0);

    for (const [ledIdx, , x, y] of LEDS) {
        if (ledIdx >= TOTAL_LEDS) continue;

        let rgb;
        if (overrideHex) {
            rgb = hexToRgb(overrideHex);
        } else if (LightingMode === "Forçado") {
            rgb = hexToRgb(forcedColor);
        } else {
            rgb = device.color(x, y); // [R, G, B] do canvas do SignalRGB
        }

        frame[ledIdx * 3]     = rgb[0];
        frame[ledIdx * 3 + 1] = rgb[1];
        frame[ledIdx * 3 + 2] = rgb[2];
    }

    // Envia via CMD DIRECT (0x12) em chunks de até 56 bytes
    // O firmware mantém o frame enquanto novos chegam; o loop do SignalRGB
    // (~30 fps) faz o keepalive automaticamente.
    evisionSend(CMD.DIRECT, 0, frame);

    // Espelha o mesmo quadro no keyglow. Falha em silêncio.
    keyglowSend(frame);
}

// ─── Espelho UDP para o keyglow ──────────────────────────────────────────────
//
// O keyglow, no modo "Do SignalRGB", ouve nesta porta e desenha o que chegar.
// Ele não manda nada ao teclado nesse modo: quem controla é este plugin.
//
// Pacote (um por quadro, little-endian):
//   [0x4B, 0x47]   "KG"
//   u8             versão (1)
//   u16 LE         número de LEDs
//   [R, G, B] × n  as mesmas cores enviadas ao teclado acima
//
// Na primeira falha o espelho se desliga de vez: o Render() roda ~30×/s e não
// pode parar por causa de um recurso que é só enfeite.

const KEYGLOW_HOST = "127.0.0.1";
const KEYGLOW_PORT = 12721;

let keyglowSocket = null;
let keyglowOff = false;

/** Escreve no log do SignalRGB, seja qual for o objeto que a versão expõe. */
function keyglowLog(msg) {
    try {
        if (typeof device !== "undefined" && typeof device.log === "function") {
            device.log("[keyglow] " + msg);
            return;
        }
    } catch (e) { /* tenta o próximo */ }
    try { console.log("[keyglow] " + msg); } catch (e) { /* desiste */ }
}

/**
 * Acha a API de UDP dentro do módulo. O nome do export mudou entre versões
 * do SignalRGB e a documentação não o fixa, então procura os formatos
 * conhecidos em vez de depender de um só.
 */
function keyglowUdpApi() {
    const candidatos = [
        udpModule && udpModule.udp,
        udpModule && udpModule.default,
        udpModule,
    ];
    for (const c of candidatos) {
        if (c && typeof c.createSocket === "function") return c;
    }
    return null;
}

/** Abre o socket e fixa o destino. Chamado uma vez, no Initialize(). */
function keyglowOpen() {
    try {
        const api = keyglowUdpApi();
        if (api === null) {
            keyglowOff = true;
            keyglowLog("módulo UDP sem createSocket() nesta versão — prévia desativada");
            return;
        }
        keyglowSocket = api.createSocket();
        keyglowSocket.connect(KEYGLOW_HOST, KEYGLOW_PORT);
        keyglowLog("espelho aberto em " + KEYGLOW_HOST + ":" + KEYGLOW_PORT);
    } catch (e) {
        keyglowOff = true;
        keyglowSocket = null;
        keyglowLog("espelho indisponível: " + e);
    }
}

function keyglowSend(frame) {
    if (keyglowOff || keyglowSocket === null) return;
    try {
        const leds = frame.length / 3;
        const pkt = new Array(5 + frame.length);
        pkt[0] = 0x4B;          // "K"
        pkt[1] = 0x47;          // "G"
        pkt[2] = 1;             // versão
        pkt[3] = leds & 0xFF;
        pkt[4] = (leds >> 8) & 0xFF;
        for (let i = 0; i < frame.length; i++) pkt[5 + i] = frame[i];
        keyglowSocket.send(pkt);
    } catch (e) {
        keyglowOff = true;
        keyglowSocket = null;
        keyglowLog("espelho desligado: " + e);
    }
}

function keyglowClose() {
    try {
        if (keyglowSocket && typeof keyglowSocket.close === "function") {
            keyglowSocket.close();
        }
    } catch (e) { /* ignora */ }
    keyglowSocket = null;
    keyglowOff = true;
}
