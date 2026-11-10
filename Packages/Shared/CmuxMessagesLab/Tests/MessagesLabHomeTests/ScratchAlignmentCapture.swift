@testable import CmuxHomeCore
import CoreGraphics
import Foundation
import ImageIO
import Testing
import UniformTypeIdentifiers
@testable import MessagesLabHome

/// SCRATCH, never lands: draws the user's scene (a Chief bubble, its agent pill, the status line) with the real row
/// drawing, light and dark, into $CAPTURE_DIR for the Before|After table.
@MainActor @Suite(.serialized) struct ScratchAlignmentCapture {
    @Test(arguments: [true, false]) func capture(light: Bool) throws {
        guard let dir = ProcessInfo.processInfo.environment["CAPTURE_DIR"] else { return }
        Fixture.lightAppearance = light
        defer { Fixture.lightAppearance = false }
        let t0 = HomeGroupColumnAlignmentTests.t0
        let bubble = HomeGroupColumnAlignmentTests.text("m1", "Cookie toast: a1076 is moving it into the new card. Cmd+I: a1026 is in Chief popover, test and land.", at: 0)
        var s = HomeGroupColumnAlignmentTests.state([bubble, try HomeGroupColumnAlignmentTests.pills(at: 1)])
        let now = t0.addingTimeInterval(40)
        var steps = HomeTurnSteps()
        steps.record(HomeLiveActivity(line: "Sending a message", since: t0.addingTimeInterval(20)))
        steps.record(HomeLiveActivity(line: "Telling an agent", since: t0.addingTimeInterval(30)))
        let progress = HomeTurnProgress(steps: steps.steps, current: HomeLiveActivity(line: "Telling an agent", since: t0.addingTimeInterval(30)),
                                        thought: "I'll post it in the thread for 25415".thoughtHeadline, now: now, style: .narrated)
        s.ui.notice = progress.text
        let width: CGFloat = 480
        let rows = HomeGroupColumnAlignmentTests.rows(s, width: width)
        let total = rows.reduce(CGFloat(24)) { $0 + $1.gap + $1.height } + 24
        let scale: CGFloat = 2
        let space = CGColorSpaceCreateDeviceRGB()
        let ctx = try #require(CGContext(data: nil, width: Int(width * scale), height: Int(total * scale), bitsPerComponent: 8,
                                         bytesPerRow: 0, space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        ctx.setFillColor(Fixture.background.cgColor)
        ctx.fill(CGRect(x: 0, y: 0, width: width * scale, height: total * scale))
        var y: CGFloat = 24
        for row in rows {
            y += row.gap
            let size = CGSize(width: row.width, height: row.height + 2 * RowDraw.margin)
            if let img = WideBitmap.make(size: size, scale: scale, opaque: false, { RowDraw.drawStatic(row, $0, windowY: .nan) }) {
                let top = y - RowDraw.margin
                ctx.draw(img, in: CGRect(x: 0, y: (total - top - size.height) * scale, width: size.width * scale, height: size.height * scale))
            }
            y += row.height
        }
        // A hairline at the bubble's left edge, so the eye can check the column.
        let edge = (Fixture.leftEdge + SenderAvatar.bubbleInset) * scale
        ctx.setFillColor(CGColor(red: 1, green: 0.2, blue: 0.4, alpha: 0.55))
        ctx.fill(CGRect(x: edge - 0.5, y: 0, width: 1, height: total * scale))
        let image = try #require(ctx.makeImage())
        let url = URL(fileURLWithPath: dir).appendingPathComponent(light ? "light.png" : "dark.png")
        try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        let dest = try #require(CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(dest, image, nil)
        #expect(CGImageDestinationFinalize(dest))
    }
}
