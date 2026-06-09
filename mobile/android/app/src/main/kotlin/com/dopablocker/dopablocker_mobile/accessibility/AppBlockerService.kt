package com.dopablocker.dopablocker_mobile.accessibility

import android.accessibilityservice.AccessibilityService
import android.content.Intent
import android.util.Log
import android.view.accessibility.AccessibilityEvent
import com.dopablocker.dopablocker_mobile.MainActivity

/// AccessibilityService que detecta a abertura de apps e bloqueia os que
/// estão na lista.
///
/// ESQUELETO (Fase M1): o roteamento de eventos de troca de janela está
/// pronto. A ação de bloqueio definitiva (overlay full-screen vs. redirect) e
/// a sincronização da lista de pacotes com o backend são TODO da Fase M2.
class AppBlockerService : AccessibilityService() {

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (event?.eventType != AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) return
        val pkg = event.packageName?.toString() ?: return
        if (pkg == packageName) return

        if (blockedPackages.contains(pkg)) {
            // TODO (Fase M2): exibir overlay de bloqueio dedicado. Por ora,
            // traz o DopaBlocker para frente, tirando o app bloqueado do foco.
            startActivity(
                Intent(this, MainActivity::class.java)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_REORDER_TO_FRONT)
            )
            Log.i(TAG, "App bloqueado detectado: $pkg")
        }
    }

    override fun onInterrupt() {
        // Sem estado contínuo para interromper.
    }

    companion object {
        private const val TAG = "AppBlockerService"

        /// Pacotes bloqueados — atualizados via MethodChannel a partir da
        /// blocklist de item_type = "app".
        @Volatile
        var blockedPackages: Set<String> = emptySet()
            private set

        fun updateBlockedPackages(packages: Set<String>) {
            blockedPackages = packages
        }
    }
}
