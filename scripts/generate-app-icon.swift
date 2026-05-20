import AppKit
import CoreGraphics
import Foundation

let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let output = root.appendingPathComponent("app-icon.png")
let publicOutput = root.appendingPathComponent("public/app-icon.png")
let size: CGFloat = 1024
let scale: CGFloat = 1

let image = NSImage(size: NSSize(width: size, height: size))
image.lockFocus()

guard let context = NSGraphicsContext.current?.cgContext else {
    fatalError("Unable to create graphics context")
}

context.setAllowsAntialiasing(true)
context.setShouldAntialias(true)
context.clear(CGRect(x: 0, y: 0, width: size, height: size))

func roundedRectPath(_ rect: CGRect, radius: CGFloat) -> CGPath {
    return CGPath(roundedRect: rect, cornerWidth: radius, cornerHeight: radius, transform: nil)
}

func fillGradient(path: CGPath, colors: [CGColor], locations: [CGFloat], start: CGPoint, end: CGPoint) {
    context.saveGState()
    context.addPath(path)
    context.clip()
    let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors as CFArray, locations: locations)!
    context.drawLinearGradient(gradient, start: start, end: end, options: [])
    context.restoreGState()
}

func color(_ red: CGFloat, _ green: CGFloat, _ blue: CGFloat, _ alpha: CGFloat = 1) -> CGColor {
    return CGColor(red: red / 255, green: green / 255, blue: blue / 255, alpha: alpha)
}

let tileRect = CGRect(x: 76, y: 76, width: 872, height: 872)
let tilePath = roundedRectPath(tileRect, radius: 190)

context.saveGState()
context.setShadow(offset: CGSize(width: 0, height: -26), blur: 44, color: color(12, 18, 32, 0.34))
context.addPath(tilePath)
context.setFillColor(color(28, 35, 52))
context.fillPath()
context.restoreGState()

fillGradient(
    path: tilePath,
    colors: [
        color(252, 254, 255),
        color(228, 239, 255),
        color(185, 217, 255)
    ],
    locations: [0, 0.56, 1],
    start: CGPoint(x: 150, y: 930),
    end: CGPoint(x: 900, y: 90)
)

context.saveGState()
context.addPath(tilePath)
context.setStrokeColor(color(255, 255, 255, 0.82))
context.setLineWidth(18)
context.strokePath()
context.restoreGState()

let insetPath = roundedRectPath(CGRect(x: 116, y: 116, width: 792, height: 792), radius: 156)
fillGradient(
    path: insetPath,
    colors: [
        color(255, 255, 255, 0.18),
        color(255, 255, 255, 0.02),
        color(20, 38, 74, 0.10)
    ],
    locations: [0, 0.58, 1],
    start: CGPoint(x: 130, y: 900),
    end: CGPoint(x: 890, y: 120)
)

let glowPath = CGMutablePath()
glowPath.move(to: CGPoint(x: 268, y: 690))
glowPath.addLine(to: CGPoint(x: 392, y: 690))
glowPath.addLine(to: CGPoint(x: 512, y: 385))
glowPath.addLine(to: CGPoint(x: 632, y: 690))
glowPath.addLine(to: CGPoint(x: 756, y: 690))
glowPath.addLine(to: CGPoint(x: 590, y: 256))
glowPath.addLine(to: CGPoint(x: 438, y: 256))
glowPath.closeSubpath()

context.saveGState()
context.setShadow(offset: CGSize(width: 0, height: -10), blur: 42, color: color(34, 124, 255, 0.28))
context.addPath(glowPath)
context.setFillColor(color(80, 140, 255, 0.42))
context.fillPath()
context.restoreGState()

let vPath = CGMutablePath()
vPath.move(to: CGPoint(x: 276, y: 708))
vPath.addLine(to: CGPoint(x: 402, y: 708))
vPath.addLine(to: CGPoint(x: 512, y: 392))
vPath.addLine(to: CGPoint(x: 622, y: 708))
vPath.addLine(to: CGPoint(x: 748, y: 708))
vPath.addLine(to: CGPoint(x: 580, y: 238))
vPath.addLine(to: CGPoint(x: 444, y: 238))
vPath.closeSubpath()

context.saveGState()
context.addPath(vPath)
context.setStrokeColor(color(255, 255, 255, 0.94))
context.setLineWidth(30)
context.setLineJoin(.round)
context.strokePath()
context.restoreGState()

fillGradient(
    path: vPath,
    colors: [
        color(255, 67, 104),
        color(255, 196, 58),
        color(71, 218, 130),
        color(48, 171, 243),
        color(135, 79, 246)
    ],
    locations: [0, 0.27, 0.50, 0.73, 1],
    start: CGPoint(x: 250, y: 740),
    end: CGPoint(x: 780, y: 250)
)

context.saveGState()
context.addPath(vPath)
context.setStrokeColor(color(255, 255, 255, 0.86))
context.setLineWidth(18)
context.setLineJoin(.round)
context.strokePath()
context.restoreGState()

context.saveGState()
let shinePath = CGMutablePath()
shinePath.move(to: CGPoint(x: 190, y: 780))
shinePath.addCurve(to: CGPoint(x: 620, y: 872), control1: CGPoint(x: 320, y: 908), control2: CGPoint(x: 500, y: 916))
shinePath.addCurve(to: CGPoint(x: 832, y: 690), control1: CGPoint(x: 730, y: 830), control2: CGPoint(x: 790, y: 760))
shinePath.addLine(to: CGPoint(x: 832, y: 848))
shinePath.addLine(to: CGPoint(x: 190, y: 848))
shinePath.closeSubpath()
context.addPath(tilePath)
context.clip()
context.addPath(shinePath)
context.setFillColor(color(255, 255, 255, 0.22))
context.fillPath()
context.restoreGState()

image.unlockFocus()

guard let tiff = image.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: tiff),
      let png = bitmap.representation(using: .png, properties: [:]) else {
    fatalError("Unable to encode PNG")
}

try png.write(to: output)
try png.write(to: publicOutput)

print("Generated \(output.path) and \(publicOutput.path) at \(Int(size * scale))x\(Int(size * scale))")
