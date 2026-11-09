import AppKit
@testable import CmuxNextApps
import CmuxNextDesign
import Testing

/// The native renderer's progress bar and menu chevron against the SwiftUI controls they replaced
/// (native-controls Before|After, controls-tour-full, 2026-11-09). In a window that is not key the
/// SwiftUI bar drew its done part grey ((170, 170, 170) in light and dark); the native bar painted the
/// primary ink in every window. The SwiftUI borderless pull-down drew its chevron in its title's ink;
/// the native one painted it secondary (title 0, chevron 134 in light). Pixels are read the way a
/// capture reads them: each control's layer rendered, over white.
@MainActor @Suite(.serialized) struct AppSceneNativeInactiveInkTests {
    private static func host() -> (NSWindow, AppSceneHostingView) {
        let model = AppSceneModel()
        model.apply([
            .create(id: "col", type: "VStack", props: ["spacing": 6]),
            .create(id: "bar", type: "ProgressView", props: ["value": 0.6]),
            .create(id: "sort", type: "Menu", props: ["title": "Sort", "menu": .array([["title": "Newest"]])]),
            .children(id: "col", children: ["bar", "sort"]),
            .root(id: "col"),
        ])
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 220, height: 120), styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.appearance = NSAppearance(named: .aqua)
        let hosting = AppSceneHostingView(model: model, bundleDirectory: nil)
        hosting.frame = NSRect(x: 0, y: 0, width: 220, height: 120)
        window.contentView = hosting
        window.orderFrontRegardless()
        hosting.layoutSubtreeIfNeeded()
        window.displayIfNeeded()
        return (window, hosting)
    }

    private static func all(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(all) }

    /// The darkest luminance (0 black, 1 white) in `view`'s layer, rendered over white, in the
    /// horizontal span `from`..`to` (fractions of its width).
    private static func darkest(_ view: NSView, from: CGFloat = 0, to: CGFloat = 1) -> CGFloat? {
        view.displayIfNeeded()
        guard let layer = view.layer else { return nil }
        let scale: CGFloat = 2
        let w = max(1, Int(ceil(view.bounds.width * scale))), h = max(1, Int(ceil(view.bounds.height * scale)))
        guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        ctx.setFillColor(CGColor(gray: 1, alpha: 1))
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        ctx.scaleBy(x: scale, y: scale)
        layer.render(in: ctx)
        guard let data = ctx.data?.assumingMemoryBound(to: UInt8.self) else { return nil }
        var best: CGFloat = 1
        for y in 0..<h {
            for x in Int(CGFloat(w) * from)..<Int(CGFloat(w) * to) {
                let p = data + (y * w + x) * 4
                let lum = (0.299 * CGFloat(p[0]) + 0.587 * CGFloat(p[1]) + 0.114 * CGFloat(p[2])) / 255
                best = min(best, lum)
            }
        }
        return best
    }

    @Test func aBarInAWindowThatIsNotKeyFillsGrey() throws {
        let (window, hosting) = Self.host()
        defer { window.contentView = nil; window.close() }
        try #require(!window.isKeyWindow, "fixture: the test window is not key")
        let bar = try #require(Self.all(hosting).first { $0 is AppSceneProgressNSView })
        let done = try #require(Self.darkest(bar, from: 0.05, to: 0.5))
        let track = try #require(Self.darkest(bar, from: 0.7, to: 0.95))
        #expect(done > 0.45, "the done part paints the primary ink (\(done)) in a window that is not key; AppKit draws it grey")
        #expect(done < track - 0.1, "the done part shows against the track: done \(done), track \(track)")
    }

    @Test func aMenuChevronTakesItsTitleInk() throws {
        let (window, hosting) = Self.host()
        defer { window.contentView = nil; window.close() }
        let menu = try #require(Self.all(hosting).first { $0 is AppSceneMenuNSView })
        let chevron = try #require(menu.subviews.first { $0 is NSImageView })
        let title = try #require(menu.subviews.first { !($0 is NSImageView) })
        let chevronInk = try #require(Self.darkest(chevron)), titleInk = try #require(Self.darkest(title))
        #expect(titleInk < 0.5, "fixture: the title drew: \(titleInk)")
        #expect(abs(chevronInk - titleInk) < 0.1, "chevron \(chevronInk), title \(titleInk)")
    }
}
