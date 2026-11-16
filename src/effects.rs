//! Efeitos de software: o app calcula uma imagem por quadro e manda ao teclado.
//!
//! Para criar um efeito novo:
//! 1. crie uma struct e implemente o trait [`Effect`] para ela;
//! 2. adicione uma entrada em [`EFFECTS`] no fim deste arquivo.

use crate::drivers::{Grid, Rgb};
use serde::{Deserialize, Serialize};

/// Parâmetros que o usuário ajusta na interface, comuns a todos os efeitos.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Params {
    pub primary: Rgb,
    pub secondary: Rgb,
    /// Multiplicador de velocidade (1.0 = normal).
    pub speed: f32,
    /// Brilho geral de 0.0 a 1.0.
    pub brightness: f32,
    /// Cores do efeito "Música + teclas": a fileira da tecla apertada e o
    /// fundo (fileira de cima e fileira de baixo).
    pub glow_key: Rgb,
    pub glow_top: Rgb,
    pub glow_bottom: Rgb,
    /// Cores do efeito "Pintura", uma por célula da grade. É salvo por
    /// teclado (veja `config`); aqui só é lido de configurações antigas.
    #[serde(skip_serializing)]
    pub paint: Vec<Rgb>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            primary: [0, 160, 255],
            secondary: [10, 0, 30],
            speed: 1.0,
            brightness: 1.0,
            glow_key: [255, 70, 0],
            glow_top: [110, 110, 110],
            glow_bottom: [140, 45, 0],
            paint: Vec::new(),
        }
    }
}

/// Faixas de frequência da análise de áudio.
pub const BANDS: usize = 21;

/// Análise do áudio que está tocando no PC.
#[derive(Clone, Copy, Debug)]
pub struct AudioData {
    /// Intensidade de 0 a 1 de cada faixa de frequência (graves à esquerda).
    pub bands: [f32; BANDS],
    /// Intensidade dos graves (0 a 1).
    pub bass: f32,
    /// Volume geral (0 a 1).
    pub level: f32,
}

impl Default for AudioData {
    fn default() -> Self {
        Self { bands: [0.0; BANDS], bass: 0.0, level: 0.0 }
    }
}

/// Tecla pressionada ou solta desde o quadro anterior.
#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    /// Código evdev da tecla (útil para efeitos que reagem a teclas específicas).
    #[allow(dead_code)]
    pub code: u16,
    /// Célula da tecla na grade, se ela já foi calibrada.
    pub led: Option<usize>,
    pub pressed: bool,
}

/// Tudo o que um efeito pode usar para desenhar um quadro.
pub struct Ctx<'a> {
    /// Segundos desde que o efeito começou.
    pub time: f32,
    /// Segundos desde o quadro anterior.
    pub dt: f32,
    pub params: &'a Params,
    /// Grade de LEDs do teclado conectado (`frame` tem `grid.len()` cores).
    pub grid: &'a Grid,
    pub events: &'a [KeyEvent],
    /// Células das teclas que estão pressionadas agora.
    pub pressed: &'a [usize],
    pub audio: &'a AudioData,
}

pub trait Effect: Send {
    fn render(&mut self, ctx: &Ctx, frame: &mut [Rgb]);

    /// Retorne `true` para o app capturar o áudio do PC.
    fn uses_audio(&self) -> bool {
        false
    }
}

/// Ajuste que um efeito usa. O texto é o rótulo mostrado na interface.
#[derive(Clone, Copy)]
pub enum Control {
    Primary(&'static str),
    Secondary(&'static str),
    /// Uma cor própria do efeito, guardada no campo de `Params` que a função devolve.
    Color(&'static str, fn(&mut Params) -> &mut Rgb),
    Speed,
}

pub struct EffectInfo {
    pub name: &'static str,
    pub description: &'static str,
    /// Ajustes que aparecem na interface (o brilho sempre aparece).
    pub controls: &'static [Control],
    pub build: fn() -> Box<dyn Effect>,
}

const TWO_COLORS: &[Control] = &[Control::Primary("Cor principal"), Control::Secondary("Cor de fundo"), Control::Speed];

// ---------------------------------------------------------------------------
// Utilidades de cor e posição

pub fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8)
}

pub fn scale(c: Rgb, k: f32) -> Rgb {
    let k = k.clamp(0.0, 1.0);
    c.map(|v| (v as f32 * k).round() as u8)
}

/// Converte HSV (matiz 0..1, saturação 0..1, valor 0..1) para RGB.
pub fn hsv(h: f32, s: f32, v: f32) -> Rgb {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

// ---------------------------------------------------------------------------
// Efeitos

/// Cada tecla com a cor pintada na interface.
struct Paint;

impl Effect for Paint {
    fn render(&mut self, ctx: &Ctx, frame: &mut [Rgb]) {
        for (i, px) in frame.iter_mut().enumerate() {
            *px = ctx.params.paint.get(i).copied().unwrap_or([0, 0, 0]);
        }
    }
}

/// Arco-íris andando da esquerda para a direita.
struct Rainbow;

impl Effect for Rainbow {
    fn render(&mut self, ctx: &Ctx, frame: &mut [Rgb]) {
        for (i, px) in frame.iter_mut().enumerate() {
            let (row, col) = ctx.grid.row_col(i);
            let h = col as f32 / ctx.grid.cols as f32 + row as f32 * 0.03 - ctx.time * 0.25 * ctx.params.speed;
            *px = hsv(h, 1.0, 1.0);
        }
    }
}

/// Teclado inteiro "respirando" entre a cor secundária e a principal.
struct Breathing;

impl Effect for Breathing {
    fn render(&mut self, ctx: &Ctx, frame: &mut [Rgb]) {
        let t = 0.5 - 0.5 * (ctx.time * 2.0 * ctx.params.speed).cos();
        frame.fill(lerp(ctx.params.secondary, ctx.params.primary, t));
    }
}

/// A tecla acende ao ser apertada e vai apagando aos poucos.
struct Reactive {
    heat: Vec<f32>,
}

impl Effect for Reactive {
    fn render(&mut self, ctx: &Ctx, frame: &mut [Rgb]) {
        self.heat.resize(frame.len(), 0.0);
        for h in self.heat.iter_mut() {
            *h = (*h - ctx.dt * 1.2 * ctx.params.speed).max(0.0);
        }
        // `events` pega toques rápidos; `pressed` mantém aceso enquanto segura.
        let tapped = ctx.events.iter().filter(|e| e.pressed).filter_map(|e| e.led);
        for led in tapped.chain(ctx.pressed.iter().copied()).filter(|&l| l < frame.len()) {
            self.heat[led] = 1.0;
        }
        for (i, px) in frame.iter_mut().enumerate() {
            *px = lerp(ctx.params.secondary, ctx.params.primary, self.heat[i]);
        }
    }
}

/// Ondas que se espalham a partir de cada tecla apertada.
struct Ripples {
    waves: Vec<(f32, f32, f32, f32)>, // (linha, coluna, início, matiz)
}

impl Effect for Ripples {
    fn render(&mut self, ctx: &Ctx, frame: &mut [Rgb]) {
        for ev in ctx.events.iter().filter(|e| e.pressed) {
            if let Some(led) = ev.led {
                let (r, c) = ctx.grid.row_col(led);
                self.waves.push((r as f32, c as f32, ctx.time, (ctx.time * 0.1).fract()));
            }
        }
        let speed = 14.0 * ctx.params.speed;
        let reach = ctx.grid.cols as f32 + 6.0;
        self.waves.retain(|w| (ctx.time - w.2) * speed < reach);

        for (i, px) in frame.iter_mut().enumerate() {
            let (r, c) = ctx.grid.row_col(i);
            let mut color = ctx.params.secondary;
            for &(wr, wc, start, _) in &self.waves {
                let radius = (ctx.time - start) * speed;
                // As linhas ficam mais afastadas que as colunas no teclado.
                let dist = ((r as f32 - wr) * 1.6).hypot(c as f32 - wc);
                let ring = (1.0 - (dist - radius).abs() / 1.8).max(0.0);
                let fade = (1.0 - radius / reach).max(0.0);
                let wave = lerp(ctx.params.secondary, ctx.params.primary, ring * fade);
                color = std::array::from_fn(|k| color[k].max(wave[k]));
            }
            *px = color;
        }
    }
}

/// Equalizador: cada coluna é uma faixa de frequência (graves à esquerda).
struct Spectrum;

impl Effect for Spectrum {
    fn uses_audio(&self) -> bool {
        true
    }

    fn render(&mut self, ctx: &Ctx, frame: &mut [Rgb]) {
        let (rows, cols) = (ctx.grid.rows, ctx.grid.cols);
        for (i, px) in frame.iter_mut().enumerate() {
            let (row, col) = ctx.grid.row_col(i);
            // Cada coluna mostra a faixa correspondente (as faixas se esticam
            // ou se juntam conforme a largura do teclado).
            let band = (col * BANDS / cols.max(1)).min(BANDS - 1);
            let height = ctx.audio.bands[band] * rows as f32;
            let from_bottom = (rows - 1 - row) as f32;
            let lit = (height - from_bottom).clamp(0.0, 1.0);
            let color = lerp(ctx.params.primary, ctx.params.secondary, from_bottom / (rows.max(2) - 1) as f32);
            *px = scale(color, lit);
        }
    }
}

/// Teclado inteiro pulsando com os graves, mudando de cor com a música.
struct BassPulse {
    hue: f32,
}

impl Effect for BassPulse {
    fn uses_audio(&self) -> bool {
        true
    }

    fn render(&mut self, ctx: &Ctx, frame: &mut [Rgb]) {
        self.hue += ctx.dt * 0.05 * ctx.params.speed + ctx.audio.bass * ctx.dt * 0.3;
        let center = ((ctx.grid.cols as f32 - 1.0) / 2.0).max(0.5);
        for (i, px) in frame.iter_mut().enumerate() {
            let (_, col) = ctx.grid.row_col(i);
            let spread = 1.0 - (col as f32 - center).abs() / center;
            let v = (ctx.audio.bass * 1.3 - (1.0 - spread) * 0.6).clamp(0.0, 1.0);
            *px = hsv(self.hue + col as f32 * 0.01, 1.0, 0.08 + 0.92 * v);
        }
    }
}

/// Música e teclas juntas, sobre uma luz de fundo em degradê (por padrão,
/// branco na fileira de cima decaindo para o laranja na de baixo).
///
/// - tocando música: o equalizador aparece por cima do fundo;
/// - digitando: a fileira da tecla apertada acende (laranja forte por padrão),
///   mais forte perto da tecla, por cima do equalizador (que continua nas outras);
/// - sem música e sem digitar: fica só o fundo.
struct MusicKeys {
    heat: Vec<f32>,
}

impl MusicKeys {
    /// Quanto a ponta mais distante da fileira acende (a tecla acende 1.0).
    const ROW_FAR: f32 = 0.35;

    /// Cor de fundo da fileira `row` (0 = a de cima).
    fn backlight(p: &Params, row: usize, rows: usize) -> Rgb {
        lerp(p.glow_top, p.glow_bottom, row as f32 / (rows.max(2) - 1) as f32)
    }

    fn new() -> Self {
        Self { heat: Vec::new() }
    }
}

impl Effect for MusicKeys {
    fn uses_audio(&self) -> bool {
        true
    }

    fn render(&mut self, ctx: &Ctx, frame: &mut [Rgb]) {
        self.heat.resize(frame.len(), 0.0);
        for h in self.heat.iter_mut() {
            *h = (*h - ctx.dt * 1.2 * ctx.params.speed).max(0.0);
        }
        let tapped = ctx.events.iter().filter(|e| e.pressed).filter_map(|e| e.led);
        // A fileira inteira da tecla acende, mais forte perto dela.
        let span = (ctx.grid.cols.max(2) - 1) as f32;
        for led in tapped.chain(ctx.pressed.iter().copied()).filter(|&l| l < frame.len()) {
            let (row, col) = ctx.grid.row_col(led);
            for c in 0..ctx.grid.cols {
                let near = 1.0 - c.abs_diff(col) as f32 / span;
                let h = &mut self.heat[row * ctx.grid.cols + c];
                *h = h.max(Self::ROW_FAR + (1.0 - Self::ROW_FAR) * near);
            }
        }

        let (rows, cols) = (ctx.grid.rows, ctx.grid.cols);
        for (i, px) in frame.iter_mut().enumerate() {
            let (row, col) = ctx.grid.row_col(i);
            let band = (col * BANDS / cols.max(1)).min(BANDS - 1);
            let height = ctx.audio.bands[band] * rows as f32;
            let from_bottom = (rows - 1 - row) as f32;
            let lit = (height - from_bottom).clamp(0.0, 1.0);
            // Do azul (graves) ao rosa (agudos), para destacar do fundo.
            let bar = hsv(0.55 + 0.4 * col as f32 / cols.max(1) as f32, 1.0, 1.0);
            *px = lerp(lerp(Self::backlight(ctx.params, row, rows), bar, lit), ctx.params.glow_key, self.heat[i]);
        }
    }
}

/// Troca sozinho de efeito: equalizador enquanto toca música, ondas quando
/// ela para.
struct AutoMusic {
    spectrum: Spectrum,
    ripples: Ripples,
    /// Segundos seguidos com som (positivo) ou em silêncio (negativo).
    streak: f32,
    playing: bool,
    /// 0.0 = ondas, 1.0 = equalizador (a troca é uma passagem suave).
    mix: f32,
    scratch: Vec<Rgb>,
}

impl AutoMusic {
    /// Volume a partir do qual conta como "tem som".
    const LEVEL: f32 = 0.01;
    /// Segundos de som para ir ao equalizador (ignora avisos curtos do sistema).
    const START: f32 = 1.0;
    /// Segundos de silêncio para voltar às ondas (ignora o intervalo entre músicas).
    const STOP: f32 = 4.0;
    /// Segundos da passagem de um efeito para o outro.
    const FADE: f32 = 0.6;

    fn new() -> Self {
        Self { spectrum: Spectrum, ripples: Ripples { waves: Vec::new() }, streak: 0.0, playing: false, mix: 0.0, scratch: Vec::new() }
    }
}

impl Effect for AutoMusic {
    fn uses_audio(&self) -> bool {
        true
    }

    fn render(&mut self, ctx: &Ctx, frame: &mut [Rgb]) {
        let sound = ctx.audio.level > Self::LEVEL;
        self.streak = match (sound, self.streak > 0.0) {
            (true, true) => self.streak + ctx.dt,
            (true, false) => ctx.dt,
            (false, false) => self.streak - ctx.dt,
            (false, true) => -ctx.dt,
        };
        if self.streak >= Self::START {
            self.playing = true;
        } else if self.streak <= -Self::STOP {
            self.playing = false;
        }
        let step = ctx.dt / Self::FADE;
        self.mix = if self.playing { (self.mix + step).min(1.0) } else { (self.mix - step).max(0.0) };

        // As ondas rodam sempre, para as teclas apertadas não se perderem na troca.
        self.ripples.render(ctx, frame);
        if self.mix > 0.0 {
            self.scratch.resize(frame.len(), [0; 3]);
            self.spectrum.render(ctx, &mut self.scratch);
            for (px, &eq) in frame.iter_mut().zip(&self.scratch) {
                *px = lerp(*px, eq, self.mix);
            }
        }
    }
}

/// Lista de efeitos que aparecem na interface.
pub const EFFECTS: &[EffectInfo] = &[
    EffectInfo {
        name: "Pintura",
        description: "Cada tecla com a cor que você pintar na tela",
        controls: &[],
        build: || Box::new(Paint),
    },
    EffectInfo {
        name: "Arco-íris",
        description: "Arco-íris atravessando o teclado",
        controls: &[Control::Speed],
        build: || Box::new(Rainbow),
    },
    EffectInfo {
        name: "Respiração",
        description: "Alterna suavemente entre as duas cores",
        controls: TWO_COLORS,
        build: || Box::new(Breathing),
    },
    EffectInfo {
        name: "Reativo",
        description: "A tecla acende ao apertar e vai apagando",
        controls: TWO_COLORS,
        build: || Box::new(Reactive { heat: Vec::new() }),
    },
    EffectInfo {
        name: "Ondas",
        description: "Ondas saindo de cada tecla apertada",
        controls: TWO_COLORS,
        build: || Box::new(Ripples { waves: Vec::new() }),
    },
    EffectInfo {
        name: "Música: equalizador",
        description: "Cada coluna é uma faixa de frequência do som que está tocando no PC",
        controls: &[Control::Primary("Cor de baixo"), Control::Secondary("Cor do topo")],
        build: || Box::new(Spectrum),
    },
    EffectInfo {
        name: "Música: pulso",
        description: "O teclado pulsa com os graves do som que está tocando no PC",
        controls: &[Control::Speed],
        build: || Box::new(BassPulse { hue: 0.0 }),
    },
    EffectInfo {
        name: "Música + teclas",
        description: "Equalizador da música e a fileira da tecla acendendo ao digitar, sobre um fundo em degradê",
        controls: &[
            Control::Color("Cor das teclas", |p| &mut p.glow_key),
            Control::Color("Fundo em cima", |p| &mut p.glow_top),
            Control::Color("Fundo embaixo", |p| &mut p.glow_bottom),
            Control::Speed,
        ],
        build: || Box::new(MusicKeys::new()),
    },
    EffectInfo {
        name: "Automático: música ou ondas",
        description: "Equalizador enquanto toca música no PC; ondas saindo das teclas quando ela para",
        controls: TWO_COLORS,
        build: || Box::new(AutoMusic::new()),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Roda `secs` segundos de um efeito a 30 quadros/s e devolve o último quadro.
    fn run(fx: &mut dyn Effect, secs: f32, audio: &AudioData) -> Vec<Rgb> {
        let (grid, params) = (Grid::full(6, 21), Params::default());
        let mut frame = vec![[0; 3]; grid.len()];
        for _ in 0..(secs * 30.0).round().max(1.0) as usize {
            let ctx = Ctx { time: 0.0, dt: 1.0 / 30.0, params: &params, grid: &grid, events: &[], pressed: &[], audio };
            fx.render(&ctx, &mut frame);
        }
        frame
    }

    #[test]
    fn auto_music_switches_between_ripples_and_equalizer() {
        let silence = AudioData::default();
        let ripples = run(&mut Ripples { waves: Vec::new() }, 0.1, &silence);
        let equalizer = run(&mut Spectrum, 0.1, &music());
        assert_ne!(ripples, equalizer);

        let mut fx = AutoMusic::new();
        assert_eq!(run(&mut fx, 1.0, &silence), ripples);
        // Um som curto não troca; música de verdade troca.
        assert_eq!(run(&mut fx, 0.5, &music()), ripples);
        assert_eq!(run(&mut fx, 0.5, &silence), ripples);
        assert_eq!(run(&mut fx, 3.0, &music()), equalizer);
        // Um intervalo curto não volta; silêncio de verdade volta.
        assert_ne!(run(&mut fx, 2.0, &silence), ripples);
        assert_eq!(run(&mut fx, 4.0, &silence), ripples);
    }

    struct Scene {
        fx: MusicKeys,
        grid: Grid,
        params: Params,
    }

    impl Scene {
        fn new() -> Self {
            Self { fx: MusicKeys::new(), grid: Grid::full(6, 21), params: Params::default() }
        }

        /// Roda `secs` segundos a 30 quadros/s e devolve o último quadro.
        fn run(&mut self, secs: f32, pressed: &[usize], audio: &AudioData) -> Vec<Rgb> {
            let mut frame = vec![[0; 3]; self.grid.len()];
            for _ in 0..(secs * 30.0).round().max(1.0) as usize {
                let ctx = Ctx { time: 0.0, dt: 1.0 / 30.0, params: &self.params, grid: &self.grid, events: &[], pressed, audio };
                self.fx.render(&ctx, &mut frame);
            }
            frame
        }
    }

    fn music() -> AudioData {
        AudioData { bands: [1.0; BANDS], bass: 1.0, level: 0.5 }
    }

    fn is_backlight(s: &Scene, frame: &[Rgb]) -> bool {
        frame.iter().enumerate().all(|(i, &px)| px == MusicKeys::backlight(&s.params, s.grid.row_col(i).0, s.grid.rows))
    }

    #[test]
    fn music_keys_backlight_is_white_on_top_and_orange_at_the_bottom() {
        let mut s = Scene::new();
        let frame = s.run(10.0, &[], &AudioData::default());
        assert!(is_backlight(&s, &frame));
        assert_eq!(frame[0], s.params.glow_top);
        assert_eq!(frame[5 * 21], s.params.glow_bottom);
        assert!(frame[2 * 21][0] > frame[0][0] && frame[2 * 21][0] < frame[5 * 21][0], "o meio fica entre as duas cores");
    }

    #[test]
    fn music_keys_key_lights_its_whole_row_in_strong_orange() {
        let mut s = Scene::new();
        // LED 40 = fileira 1, coluna 19.
        let frame = s.run(0.5, &[40], &AudioData::default());
        assert_eq!(frame[40], s.params.glow_key);
        let row_bg = MusicKeys::backlight(&s.params, 1, 6);
        assert!(frame[21..42].iter().all(|&px| px != row_bg), "a fileira inteira reage");
        assert!(frame[39][2] < frame[21][2], "mais forte perto da tecla");
        assert_eq!(frame[0], s.params.glow_top, "as outras fileiras não mudam");
        assert_eq!(frame[42], MusicKeys::backlight(&s.params, 2, 6));
    }

    #[test]
    fn music_keys_keeps_the_bars_on_other_rows_while_typing() {
        let mut s = Scene::new();
        let frame = s.run(1.0, &[], &music());
        let bar = |i: usize| hsv(0.55 + 0.4 * (i % 21) as f32 / 21.0, 1.0, 1.0);
        assert!(frame.iter().enumerate().all(|(i, &px)| px == bar(i)), "com todas as faixas no máximo, tudo é barra");
        // Digitando: a fileira da tecla acende e as outras seguem com a música.
        let frame = s.run(0.5, &[40], &music());
        assert_eq!(frame[40], s.params.glow_key);
        assert!((0..126).filter(|i| !(21..42).contains(i)).all(|i| frame[i] == bar(i)));
        // Parou de digitar: a fileira volta para a música.
        let frame = s.run(3.0, &[], &music());
        assert!(frame.iter().enumerate().all(|(i, &px)| px == bar(i)));
        // Parou a música: fica só o fundo.
        let frame = s.run(1.0, &[], &AudioData::default());
        assert!(is_backlight(&s, &frame));
    }
}
