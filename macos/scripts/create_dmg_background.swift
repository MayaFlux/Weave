import AppKit
import Foundation

guard CommandLine.arguments.count == 2 else {
    fatalError("Expected output PNG path")
}

let outputURL = URL(fileURLWithPath: CommandLine.arguments[1])
let image = NSBitmapImageRep(
    bitmapDataPlanes: nil,
    pixelsWide: 1600,
    pixelsHigh: 800,
    bitsPerSample: 8,
    samplesPerPixel: 4,
    hasAlpha: true,
    isPlanar: false,
    colorSpaceName: .deviceRGB,
    bytesPerRow: 0,
    bitsPerPixel: 0
)!
image.size = NSSize(width: 800, height: 400)

guard let graphicsContext = NSGraphicsContext(bitmapImageRep: image) else {
    fatalError("Could not create DMG background graphics context")
}
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = graphicsContext
NSColor(calibratedRed: 0.075, green: 0.09, blue: 0.13, alpha: 1).setFill()
NSBezierPath(rect: NSRect(x: 0, y: 0, width: 800, height: 400)).fill()

let arrow = NSBezierPath()
arrow.lineWidth = 3
arrow.lineCapStyle = .round
arrow.lineJoinStyle = .round
arrow.move(to: NSPoint(x: 365, y: 200))
arrow.line(to: NSPoint(x: 435, y: 200))
arrow.move(to: NSPoint(x: 420, y: 215))
arrow.line(to: NSPoint(x: 435, y: 200))
arrow.line(to: NSPoint(x: 420, y: 185))
NSColor(calibratedRed: 0.72, green: 0.82, blue: 1, alpha: 0.92).setStroke()
arrow.stroke()

graphicsContext.flushGraphics()
NSGraphicsContext.restoreGraphicsState()

guard let png = image.representation(using: .png, properties: [:]) else {
    fatalError("Could not encode DMG background PNG")
}
try png.write(to: outputURL, options: .atomic)
