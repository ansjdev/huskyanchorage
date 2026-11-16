// Desenha o ícone do keyglow (PNG 1024x1024): uma tecla acesa num fundo escuro.
// Uso: swift scripts/icone-macos.swift saida.png
import AppKit

let size = 1024
let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "icone.png"

let rep = NSBitmapImageRep(
    bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8,
    samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
    bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
let ctx = NSGraphicsContext.current!.cgContext
let space = CGColorSpaceCreateDeviceRGB()

func rgb(_ r: CGFloat, _ g: CGFloat, _ b: CGFloat, _ a: CGFloat = 1) -> CGColor {
    CGColor(colorSpace: space, components: [r / 255, g / 255, b / 255, a])!
}

// Fundo: quadrado arredondado no padrão do macOS (margem de 100 px em 1024).
let plate = CGRect(x: 100, y: 100, width: 824, height: 824)
let platePath = CGPath(roundedRect: plate, cornerWidth: 185, cornerHeight: 185, transform: nil)
ctx.saveGState()
ctx.addPath(platePath)
ctx.clip()
let bg = CGGradient(colorsSpace: space, colors: [rgb(28, 30, 44), rgb(10, 11, 18)] as CFArray, locations: [0, 1])!
ctx.drawLinearGradient(bg, start: CGPoint(x: 512, y: 924), end: CGPoint(x: 512, y: 100), options: [])

// Brilho colorido saindo de baixo da tecla.
let glows: [(CGPoint, CGColor)] = [
    (CGPoint(x: 330, y: 400), rgb(255, 60, 140, 0.85)),
    (CGPoint(x: 512, y: 350), rgb(120, 90, 255, 0.85)),
    (CGPoint(x: 694, y: 400), rgb(0, 210, 255, 0.85)),
]
ctx.setBlendMode(.screen)
for (center, color) in glows {
    let g = CGGradient(colorsSpace: space, colors: [color, color.copy(alpha: 0)!] as CFArray, locations: [0, 1])!
    ctx.drawRadialGradient(g, startCenter: center, startRadius: 0, endCenter: center, endRadius: 380, options: [])
}
ctx.setBlendMode(.normal)
ctx.restoreGState()

// Tecla: base escura e topo mais claro.
let base = CGRect(x: 282, y: 300, width: 460, height: 440)
ctx.addPath(CGPath(roundedRect: base, cornerWidth: 80, cornerHeight: 80, transform: nil))
ctx.setFillColor(rgb(16, 17, 26))
ctx.fillPath()

let top = CGRect(x: 318, y: 366, width: 388, height: 344)
let topPath = CGPath(roundedRect: top, cornerWidth: 58, cornerHeight: 58, transform: nil)
ctx.saveGState()
ctx.addPath(topPath)
ctx.clip()
let cap = CGGradient(colorsSpace: space, colors: [rgb(58, 61, 84), rgb(34, 36, 52)] as CFArray, locations: [0, 1])!
ctx.drawLinearGradient(cap, start: CGPoint(x: 512, y: 710), end: CGPoint(x: 512, y: 366), options: [])
ctx.restoreGState()

// Letra K acesa.
let font = NSFont.systemFont(ofSize: 250, weight: .bold)
let shadow = NSShadow()
shadow.shadowColor = NSColor(red: 0.45, green: 0.75, blue: 1, alpha: 0.9)
shadow.shadowBlurRadius = 40
let text = NSAttributedString(string: "K", attributes: [
    .font: font, .foregroundColor: NSColor.white, .shadow: shadow,
])
let ts = text.size()
text.draw(at: NSPoint(x: top.midX - ts.width / 2, y: top.midY - ts.height / 2))

NSGraphicsContext.current!.flushGraphics()
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: out))
