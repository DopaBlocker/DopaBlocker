package com.dopablocker.dopablocker_mobile.accessibility

import android.accessibilityservice.AccessibilityService
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.accessibility.AccessibilityEvent
import com.dopablocker.dopablocker_mobile.MainActivity
import com.dopablocker.dopablocker_mobile.TamperReporter
import com.dopablocker.dopablocker_mobile.vpn.DnsVpnService
import java.util.concurrent.ConcurrentHashMap

/// AccessibilityService que (a) bloqueia apps da lista com um overlay
/// full-screen (C3), (b) mostra o overlay de **site bloqueado** quando a VPN
/// sinaliza um domínio bloqueado num navegador em foco (C1) e (c) detecta a
/// abertura das Configs de VPN/DNS como evento de adulteração (C2.2),
/// reportando ao backend para o pai ver.
class AppBlockerService : AccessibilityService() {

    /// Debounce do report de tamper — evita spam ao navegar nas Configurações.
    private var lastTamperReportMs = 0L

    /// Para postar `startActivity`/`performGlobalAction` na main thread quando a
    /// chamada vem da thread da VPN (`notifyBlockedDomain`).
    private val mainHandler = Handler(Looper.getMainLooper())

    /// Pacotes de navegadores (cache). Computado sob demanda via PackageManager
    /// para gatear o overlay de site só quando o usuário está num navegador.
    private var browserPackagesCache: Set<String>? = null

    /// Debounce do overlay de site por domínio — uma página dispara várias
    /// queries (CDN, analytics); evita repetir o overlay.
    private val recentSiteOverlays = ConcurrentHashMap<String, Long>()

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
        browserPackagesCache = null
        // Recarrega a lista persistida (cobre reinício do serviço pelo sistema).
        loadBlockedPackages(this)
    }

    override fun onUnbind(intent: Intent?): Boolean {
        if (instance === this) instance = null
        return super.onUnbind(intent)
    }

    override fun onDestroy() {
        if (instance === this) instance = null
        super.onDestroy()
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (event?.eventType != AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) return
        val pkg = event.packageName?.toString() ?: return
        if (pkg == packageName) return

        // Rastreia o app em foreground (usado para gatear o overlay de site).
        lastForegroundPackage = pkg

        // (C2.2) Adulteração: abriu Configs de VPN/DNS → traz o app + reporta.
        val tamperKind = SettingsTamperDetector.kindFor(pkg, event.className?.toString().orEmpty())
        if (tamperKind != null) {
            bringAppToFront()
            maybeReportTamper(tamperKind)
            return
        }

        // (C3) App bloqueado → overlay full-screen + dispensa o app do foreground
        // (efeito "abre e fecha sozinho"). HOME primeiro para o app sair de cena,
        // depois o overlay vem por cima do launcher.
        if (blockedPackages.contains(pkg)) {
            performGlobalAction(GLOBAL_ACTION_HOME)
            launchOverlay(BlockOverlayActivity.KIND_APP, pkg = pkg, domain = null)
            Log.i(TAG, "App bloqueado: $pkg")
        }
    }

    override fun onInterrupt() {
        // Sem estado contínuo para interromper.
    }

    /// Chamado pela VPN (mesmo processo, outra thread) quando um domínio é
    /// bloqueado. Mostra o overlay de site só se o app em foreground for um
    /// navegador, com debounce por domínio — evita overlays espúrios de trackers
    /// em background.
    fun notifyBlockedDomain(domain: String) {
        if (domain.isBlank()) return
        val fg = lastForegroundPackage ?: return
        if (!browserPackages().contains(fg)) return

        val now = System.currentTimeMillis()
        val last = recentSiteOverlays[domain] ?: 0L
        if (now - last < SITE_DEBOUNCE_MS) return
        recentSiteOverlays[domain] = now

        mainHandler.post { launchOverlay(BlockOverlayActivity.KIND_SITE, pkg = null, domain = domain) }
        Log.i(TAG, "Site bloqueado (overlay): $domain")
    }

    private fun launchOverlay(kind: String, pkg: String?, domain: String?) {
        startActivity(
            Intent(this, BlockOverlayActivity::class.java).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
                putExtra(BlockOverlayActivity.EXTRA_KIND, kind)
                if (pkg != null) putExtra(BlockOverlayActivity.EXTRA_PACKAGE, pkg)
                if (domain != null) putExtra(BlockOverlayActivity.EXTRA_DOMAIN, domain)
            },
        )
    }

    /// Resolve (e cacheia) os pacotes que respondem a `https VIEW BROWSABLE` —
    /// i.e. navegadores. Exige o bloco <queries> correspondente no manifesto.
    private fun browserPackages(): Set<String> {
        browserPackagesCache?.let { return it }
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse("https://example.com"))
            .addCategory(Intent.CATEGORY_BROWSABLE)
        val resolved = runCatching { packageManager.queryIntentActivities(intent, 0) }.getOrNull()
        val set = resolved
            ?.mapNotNull { it.activityInfo?.packageName }
            ?.toSet()
            ?: emptySet()
        browserPackagesCache = set
        return set
    }

    private fun bringAppToFront() {
        startActivity(
            Intent(this, MainActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_REORDER_TO_FRONT),
        )
    }

    private fun maybeReportTamper(kind: String) {
        val now = System.currentTimeMillis()
        if (now - lastTamperReportMs < TAMPER_DEBOUNCE_MS) return
        lastTamperReportMs = now
        TamperReporter.report(this, kind)
    }

    companion object {
        private const val TAG = "AppBlockerService"
        private const val TAMPER_DEBOUNCE_MS = 10_000L
        private const val SITE_DEBOUNCE_MS = 5_000L

        /// Instância viva do serviço (ou null se desativado). A VPN, no mesmo
        /// processo, usa isto para disparar o overlay de site. `@Volatile`:
        /// escrito no lifecycle (main), lido pela thread da VPN.
        @Volatile
        var instance: AppBlockerService? = null
            private set

        /// Último app que ganhou foco (window-state-changed). Usado para gatear o
        /// overlay de site (só em navegador). `@Volatile`: escrito na main, lido
        /// pela thread da VPN.
        @Volatile
        var lastForegroundPackage: String? = null
            private set

        /// Chave no SharedPreferences compartilhado (`DnsVpnService.PREFS`).
        const val KEY_BLOCKED_APPS = "blocked_apps"

        /// Pacotes bloqueados — atualizados via MethodChannel a partir da
        /// blocklist de `item_type = "app"`. `@Volatile`: lido no callback de
        /// eventos, escrito pelo MethodChannel.
        @Volatile
        var blockedPackages: Set<String> = emptySet()
            private set

        private fun prefs(context: Context) =
            context.getSharedPreferences(DnsVpnService.PREFS, Context.MODE_PRIVATE)

        /// Atualiza a lista de pacotes bloqueados (runtime + persistência). A
        /// persistência cobre o restart do serviço e o boot.
        fun updateBlockedPackages(context: Context, packages: Set<String>) {
            blockedPackages = packages
            prefs(context).edit().putStringSet(KEY_BLOCKED_APPS, packages).apply()
        }

        /// Recarrega a lista persistida para o runtime.
        fun loadBlockedPackages(context: Context) {
            blockedPackages =
                prefs(context).getStringSet(KEY_BLOCKED_APPS, emptySet())?.toSet() ?: emptySet()
        }
    }
}
