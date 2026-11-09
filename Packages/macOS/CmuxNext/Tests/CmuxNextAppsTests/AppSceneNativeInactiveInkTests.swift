import AppKit
@testable import CmuxNextApps
import CmuxNextDesign
import Testing

/// The native renderer's progress bar and menu chevron against the SwiftUI and AppKit controls they
/// replaced (native-controls Before|After, 2026-11-09). AppKit draws a bar's done part grey while its
/// window is not key (the Before capture, from a window that is not key: (170, 170, 170) in light and
/// dark); the native bar painted the primary ink in every window. A borderless pull-down draws its
/// chevron in its title's ink; the native one painted it secondary.
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
        hosting.layoutSubtreeIfNeeded()
        hosting.displayIfNeeded()
        return (window, hosting)
    }

    private static func all(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(all) }

    private static func luminance(_ color: CGColor?) -> CGFloat? {
        guard let color, let rgb = NSColor(cgColor: color)?.usingColorSpace(.sRGB) else { return nil }
        return (0.299 * rgb.redComponent + 0.587 * rgb.greenComponent + 0.114 * rgb.blueComponent) * rgb.alphaComponent
            + (1 - rgb.alphaComponent)
    }

    @Test func aBarInAWindowThatIsNotKeyFillsGreyNotPrimaryInk() throws {
        let (window, hosting) = Self.host()
        defer { window.contentView = nil; window.close() }
        try #require(!window.isKeyWindow, "fixture: a test window is not key")
        let bar = try #require(Self.all(hosting).first { $0 is AppSceneProgressNSView })
        bar.displayIfNeeded()
        // The bar's two rounded layers: the track across the width and the done part at 60% of it.
        let bars = (bar.layer?.sublayers ?? []).filter { $0.cornerRadius > 0 && $0.frame.height > 0 }.sorted { $0.frame.width < $1.frame.width }
        let done = try #require(bars.first, "no done layer: \(String(describing: bar.layer?.sublayers))")
        let fill = try #require(Self.luminance(done.backgroundColor), "the done part has no fill")
        let primary = try #require(Self.luminance(window.effectiveAppearance.performAsCurrentDrawingAppearance { Palette.textPrimary.cgColor }))
        #expect(fill > primary + 0.25, "the done part paints the primary ink (\(fill), primary \(primary)) in a window that is not key")
        #expect(fill < 0.9, "the done part still shows against the track: \(fill)")
    }

    @Test func aMenuChevronTakesItsTitleInk() throws {
        let (window, hosting) = Self.host()
        defer { window.contentView = nil; window.close() }
        let menu = try #require(Self.all(hosting).first { $0 is AppSceneMenuNSView })
        menu.displayIfNeeded()
        let chevron = try #require(menu.subviews.compactMap { $0 as? NSImageView }.first)
        let title = try #require(menu.subviews.compactMap { $0 as? NSTextField }.first)
        let ink = { (color: NSColor?) in window.effectiveAppearance.performAsCurrentDrawingAppearance { Self.luminance(color?.cgColor) } }
        let chevronInk = try #require(ink(chevron.contentTintColor)), titleInk = try #require(ink(title.textColor))
        #expect(abs(chevronInk - titleInk) < 0.02, "chevron \(chevronInk), title \(titleInk)")
    }
}
