import Foundation
import CoreGraphics

guard CommandLine.arguments.count >= 2, let pid = Int32(CommandLine.arguments[1]) else {
    fputs("usage: macos-window-id.swift <pid>\n", stderr)
    exit(2)
}

guard let raw = CGWindowListCopyWindowInfo([], kCGNullWindowID) as? [[String: Any]] else {
    fputs("no window list\n", stderr)
    exit(1)
}

var best: (number: Int, area: Int, x: Int, y: Int, width: Int, height: Int)?
for window in raw {
    guard let owner = window[kCGWindowOwnerPID as String] as? pid_t, owner == pid else { continue }
    let bounds = window[kCGWindowBounds as String] as? [String: Any]
    let number = window[kCGWindowNumber as String] as? Int ?? 0
    let x = Int((bounds?["X"] as? NSNumber)?.doubleValue ?? 0)
    let y = Int((bounds?["Y"] as? NSNumber)?.doubleValue ?? 0)
    let width = Int((bounds?["Width"] as? NSNumber)?.doubleValue ?? 0)
    let height = Int((bounds?["Height"] as? NSNumber)?.doubleValue ?? 0)
    let area = max(width, 0) * max(height, 0)
    fputs("window \(number) \(x),\(y) \(width)x\(height) layer=\(window[kCGWindowLayer as String] ?? 0)\n", stderr)
    if best == nil || area > best!.area {
        best = (number, area, x, y, width, height)
    }
}

guard let best else {
    fputs("window not found for pid \(pid)\n", stderr)
    exit(1)
}

print("\(best.number) \(best.x) \(best.y) \(best.width) \(best.height)")
