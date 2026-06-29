package com.dopablocker.dopablocker_mobile.channel

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.net.VpnService
import android.os.Build
import android.provider.Settings
import com.dopablocker.dopablocker_mobile.accessibility.AppBlockerService
import com.dopablocker.dopablocker_mobile.apps.InstalledAppsProvider
import com.dopablocker.dopablocker_mobile.reporting.TamperReporter
import com.dopablocker.dopablocker_mobile.vpn.DnsVpnService
import com.dopablocker.dopablocker_mobile.vpn.VpnManager
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

/// Handler do MethodChannel `com.dopablocker/blocking` (espelha
/// `lib/core/channels/blocking_channel.dart`). Mapeia cada método para os
/// serviços nativos (VpnManager / DnsVpnService / AppBlockerService) e cuida do
/// fluxo de consentimento de VPN, que precisa de um `onActivityResult` — por
/// isso recebe a [Activity] e é alimentado pelo `MainActivity.onActivityResult`
/// via [onVpnConsentResult].
class BlockingChannelHandler(private val activity: Activity) : MethodChannel.MethodCallHandler {

    private val vpnRequestCode = 0x0F01
    private var pendingResult: MethodChannel.Result? = null

    private val context get() = activity.applicationContext

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            BlockingMethods.START_VPN -> startVpn(result)
            BlockingMethods.STOP_VPN -> {
                VpnManager.stop(activity)
                result.success(true)
            }
            BlockingMethods.IS_VPN_ACTIVE -> result.success(VpnManager.isActive())
            // Consentimento de VPN já concedido? `prepare()` devolve null quando
            // não é mais preciso pedir — usado pelo muro obrigatório do filho
            // para saber se a etapa de VPN já está cumprida.
            BlockingMethods.IS_VPN_PREPARED -> result.success(VpnService.prepare(activity) == null)
            BlockingMethods.UPDATE_BLOCKLIST -> {
                val domains = call.argument<List<String>>("domains") ?: emptyList()
                DnsVpnService.updateBlocklist(context, domains)
                result.success(true)
            }
            BlockingMethods.UPDATE_BLOCKED_APPS -> {
                val packages = call.argument<List<String>>("packages") ?: emptyList()
                AppBlockerService.updateBlockedPackages(context, packages.toSet())
                result.success(true)
            }
            BlockingMethods.SET_ADULT_FILTER -> {
                val enabled = call.argument<Boolean>("enabled") ?: false
                DnsVpnService.setAdultFilter(context, enabled)
                result.success(true)
            }
            BlockingMethods.SET_TAMPER_CONFIG -> {
                TamperReporter.setConfig(
                    context,
                    call.argument<String>("deviceToken"),
                    call.argument<String>("backendUrl"),
                    call.argument<Boolean>("isChild") ?: false,
                )
                result.success(true)
            }
            BlockingMethods.GET_INSTALLED_APPS -> {
                // Enumeração + decode de ícones é pesada: roda fora da main
                // thread e responde de volta nela.
                Thread {
                    val apps = InstalledAppsProvider.listLaunchableApps(context)
                    activity.runOnUiThread { result.success(apps) }
                }.start()
            }
            BlockingMethods.IS_ACCESSIBILITY_ENABLED -> result.success(isAccessibilityEnabled())
            BlockingMethods.OPEN_ACCESSIBILITY_SETTINGS -> {
                activity.startActivity(
                    Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                )
                result.success(true)
            }
            BlockingMethods.CAN_DRAW_OVERLAYS -> {
                // Antes do Android 6 (M) a permissão é concedida no install.
                val ok = Build.VERSION.SDK_INT < Build.VERSION_CODES.M ||
                    Settings.canDrawOverlays(activity)
                result.success(ok)
            }
            BlockingMethods.REQUEST_OVERLAY_PERMISSION -> {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    activity.startActivity(
                        Intent(
                            Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                            Uri.parse("package:${activity.packageName}"),
                        ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    )
                }
                result.success(true)
            }
            else -> result.notImplemented()
        }
    }

    /// Inicia a VPN. Se o usuário ainda não consentiu, o Android exige abrir o
    /// diálogo de permissão de VPN — só após RESULT_OK o serviço sobe (resolvido
    /// em [onVpnConsentResult]).
    private fun startVpn(result: MethodChannel.Result) {
        val consentIntent = VpnService.prepare(activity)
        if (consentIntent != null) {
            pendingResult = result
            activity.startActivityForResult(consentIntent, vpnRequestCode)
        } else {
            VpnManager.start(activity)
            result.success(true)
        }
    }

    /// Repassado pelo `MainActivity.onActivityResult`. Resolve o `startVpn`
    /// pendente com o resultado do consentimento. Devolve `true` se tratou o
    /// `requestCode` (era o do consentimento de VPN).
    fun onVpnConsentResult(requestCode: Int, resultCode: Int): Boolean {
        if (requestCode != vpnRequestCode) return false
        val granted = resultCode == Activity.RESULT_OK
        if (granted) VpnManager.start(activity)
        pendingResult?.success(granted)
        pendingResult = null
        return true
    }

    /// Lê em Settings.Secure se o AppBlockerService está habilitado pelo usuário.
    private fun isAccessibilityEnabled(): Boolean {
        val expected = "${activity.packageName}/${AppBlockerService::class.java.name}"
        val enabledServices = Settings.Secure.getString(
            activity.contentResolver,
            Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
        ) ?: return false
        return enabledServices.split(':').any { it.equals(expected, ignoreCase = true) }
    }
}
