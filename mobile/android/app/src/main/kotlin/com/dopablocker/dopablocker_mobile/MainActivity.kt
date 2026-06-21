package com.dopablocker.dopablocker_mobile

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import android.provider.Settings
import com.dopablocker.dopablocker_mobile.accessibility.AppBlockerService
import com.dopablocker.dopablocker_mobile.vpn.DnsVpnService
import com.dopablocker.dopablocker_mobile.vpn.VpnManager
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

/// Ponto de entrada Android. Registra o MethodChannel `com.dopablocker/blocking`
/// (espelha lib/channels/blocking_channel.dart) e delega para os serviços
/// nativos VpnManager / DnsVpnService / AppBlockerService.
class MainActivity : FlutterActivity() {

    private val channelName = "com.dopablocker/blocking"
    private val vpnRequestCode = 0x0F01
    private var pendingResult: MethodChannel.Result? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "startVpn" -> startVpn(result)
                    "stopVpn" -> {
                        VpnManager.stop(this)
                        result.success(true)
                    }
                    "isVpnActive" -> result.success(VpnManager.isActive())
                    "updateBlocklist" -> {
                        val domains = call.argument<List<String>>("domains") ?: emptyList()
                        DnsVpnService.updateBlocklist(applicationContext, domains)
                        result.success(true)
                    }
                    "isAccessibilityEnabled" -> result.success(isAccessibilityEnabled())
                    "openAccessibilitySettings" -> {
                        startActivity(
                            Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)
                                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                        )
                        result.success(true)
                    }
                    else -> result.notImplemented()
                }
            }
    }

    /// Inicia a VPN. Se o usuário ainda não consentiu, o Android exige abrir o
    /// diálogo de permissão de VPN — só após RESULT_OK o serviço sobe.
    private fun startVpn(result: MethodChannel.Result) {
        val consentIntent = VpnService.prepare(this)
        if (consentIntent != null) {
            pendingResult = result
            startActivityForResult(consentIntent, vpnRequestCode)
        } else {
            VpnManager.start(this)
            result.success(true)
        }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == vpnRequestCode) {
            val granted = resultCode == Activity.RESULT_OK
            if (granted) VpnManager.start(this)
            pendingResult?.success(granted)
            pendingResult = null
        }
    }

    /// Lê em Settings.Secure se o AppBlockerService está habilitado pelo usuário.
    private fun isAccessibilityEnabled(): Boolean {
        val expected = "$packageName/${AppBlockerService::class.java.name}"
        val enabledServices = Settings.Secure.getString(
            contentResolver,
            Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
        ) ?: return false
        return enabledServices.split(':').any { it.equals(expected, ignoreCase = true) }
    }
}
