public import AppKit
public import CmuxNextDesign

// The floating Chief's own window moves from a press on the chat's empty space, and only there (user 2026-11-09):
// the transcript says where that is; its notice and first-run card are views with their own rules.
extension HomeNativeTranscriptView: WindowBackgroundDragDeciding {
    /// Whether a press at `point` (window coordinates) on `hit` moves the floating Chief's window: on the transcript,
    /// only its empty space between messages; nil elsewhere.
    public func pressMovesWindow(atWindowPoint point: CGPoint, hit: NSView) -> Bool? {
        hit.isDescendant(of: transcript) ? transcript.pressMovesWindow(atWindowPoint: point, hit: hit) : nil
    }
}
