#!/bin/bash
# Render the editable SVG into a macOS iconset and ICNS bundle resource.
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
SVG="$ROOT/resources/icon/DooTube.svg"
TEMP_DIR="$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/DooTube.XXXXXX")"
ICONSET="$TEMP_DIR/DooTube.iconset"
/bin/mkdir "$ICONSET"
trap '/bin/rm -rf "$TEMP_DIR"' EXIT

/usr/bin/swift - "$SVG" "$ICONSET" <<'SWIFT'
import AppKit
import Foundation

let svgURL = URL(fileURLWithPath: CommandLine.arguments[1])
let output = URL(fileURLWithPath: CommandLine.arguments[2])
guard let image = NSImage(contentsOf: svgURL) else {
    fatalError("Could not load SVG: \(svgURL.path)")
}
for size in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let pixels = size * scale
        let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
                                   bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
                                   isPlanar: false, colorSpaceName: .deviceRGB,
                                   bytesPerRow: 0, bitsPerPixel: 0)!
        rep.size = NSSize(width: pixels, height: pixels)
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
        NSGraphicsContext.current?.imageInterpolation = .high
        image.draw(in: NSRect(x: 0, y: 0, width: pixels, height: pixels),
                   from: .zero, operation: .copy, fraction: 1)
        NSGraphicsContext.restoreGraphicsState()
        let suffix = scale == 1 ? "" : "@2x"
        let url = output.appendingPathComponent("icon_\(size)x\(size)\(suffix).png")
        try rep.representation(using: .png, properties: [:])!.write(to: url)
    }
}
SWIFT
/usr/bin/swift - "$ICONSET" "$ROOT/resources/icon/preview-small.png" <<'SWIFT'
import AppKit
import Foundation

let source = URL(fileURLWithPath: CommandLine.arguments[1])
let output = URL(fileURLWithPath: CommandLine.arguments[2])
let sizes = [16, 32, 64, 128]
let gap = 16
let height = 128
let width = sizes.count * height + gap * (sizes.count - 1)
let strip = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height,
                             bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
                             isPlanar: false, colorSpaceName: .deviceRGB,
                             bytesPerRow: 0, bitsPerPixel: 0)!
strip.size = NSSize(width: width, height: height)
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: strip)
NSGraphicsContext.current?.imageInterpolation = .none
var x = 0
for size in sizes {
    let filename: String
    if size == 64 {
        filename = "icon_32x32@2x.png"
    } else {
        filename = "icon_\(size)x\(size).png"
    }
    guard let image = NSImage(contentsOf: source.appendingPathComponent(filename)) else {
        fatalError("Could not read \(filename)")
    }
    image.draw(in: NSRect(x: x, y: 0, width: height, height: height),
               from: .zero, operation: .copy, fraction: 1)
    x += height + gap
}
NSGraphicsContext.restoreGraphicsState()
try strip.representation(using: .png, properties: [:])!.write(to: output)
SWIFT

/usr/bin/iconutil -c icns "$ICONSET" -o "$ROOT/resources/DooTube.icns"
/usr/bin/sips -s format png -z 512 512 "$ICONSET/icon_512x512.png" --out "$ROOT/resources/icon/preview-512.png" >/dev/null

