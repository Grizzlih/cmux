import AppKit
@testable import CmuxNextApp
import CmuxNextDaemon
import CmuxNextDaemonService
import Testing

/// Naming a restored New Tab page reads no tab list (Lawrence, 2026-11-09: "pretty bad lag just
/// scrolling the Chief area"). The app-wide link-place list names every tab of every workspace on
/// each change; each New Tab page's title asked `AgentHistoryPage.title` by bare id, whose key
/// lookup walked every chat of every tree (`listTabsScoped`) once per tab: quadratic, 64% of the
/// main thread with 824 workspaces open.
@MainActor @Suite(.serialized, .closesItsWindows)
struct TabTitleNoTabListReadTests {
    @Test func namingNewTabPagesReadsNoTabList() {
        _ = NSApplication.shared
        let services = AppServices(environment: AppEnvironment.current([:]))
        // Fifty workspaces, each with one restored New Tab page (an agent tab with no session yet).
        let workspaces = (UInt64(1)...50).map { index in
            var page = TabSnapshot(surface: SurfaceID(rawValue: index), kind: .conversation, title: "about:blank", url: "about:blank")
            page.conversation = ConversationTabRef(agentSession: AgentSessionRef(host: "local", harness: "claude"))
            let screen = ScreenSnapshot(id: ScreenID(rawValue: index), layout: .leaf(PaneID(rawValue: index)), panes: [PaneSnapshot(id: PaneID(rawValue: index), tabs: [page])])
            return WorkspaceSnapshot(id: WorkspaceHandle(rawValue: index), key: nil, name: "w\(index)", screens: [screen])
        }
        services.machines.local.store.apply(snapshot: DaemonTree(workspaces: workspaces))
        let tabs = services.machines.local.store.workspaces.flatMap(\.screens).flatMap(\.panes).flatMap(\.tabs)
        #expect(tabs.count == 50)
        var listed = 0
        let list = services.agentTabs.listTabsScoped
        services.agentTabs.listTabsScoped = { listed += 1; return list() }
        let resolver = TabTitleResolver(services: services)
        for tab in tabs { #expect(resolver.title(tab) == Strings.untitledBrowser) }
        #expect(listed == 0, "each New Tab page's title walked every tab: \(listed) list reads for \(tabs.count) tabs")
    }
}
