package com.dopablocker.dopablocker_mobile.reporting

import android.content.Context
import android.util.Log
import com.dopablocker.dopablocker_mobile.vpn.DnsVpnService
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import kotlin.concurrent.thread

/// Reporta eventos de adulteração (tamper) ao backend (C2.1/C2.2). Sem root, o
/// filho consegue desligar a VPN ou abrir as Configs de VPN/DNS; não dá para
/// IMPEDIR, mas dá para TORNAR VISÍVEL ao responsável.
///
/// Autenticação: manda o Device Token NO CORPO para `POST /devices/tamper`
/// (rota pública auto-autenticada — ver backend), porque o lado nativo não lê
/// o `flutter_secure_storage` do Dart. Por isso o token/URL/flag são gravados
/// nas SharedPreferences pelo Dart (via `setTamperConfig`) ao entrar na sessão
/// de filho. É best-effort: roda numa thread curta e ignora falhas de rede.
object TamperReporter {

    private const val TAG = "TamperReporter"
    private const val TIMEOUT_MS = 5000

    const val KEY_DEVICE_TOKEN = "tamper_device_token"
    const val KEY_BACKEND_URL = "tamper_backend_url"
    const val KEY_IS_CHILD = "tamper_is_child"

    private fun prefs(context: Context) =
        context.getSharedPreferences(DnsVpnService.PREFS, Context.MODE_PRIVATE)

    /// Grava (ou limpa) a config usada pelo report. Chamado pelo Dart ao
    /// estabelecer/encerrar a sessão de filho.
    fun setConfig(context: Context, deviceToken: String?, backendUrl: String?, isChild: Boolean) {
        prefs(context).edit().apply {
            if (deviceToken.isNullOrBlank()) remove(KEY_DEVICE_TOKEN)
            else putString(KEY_DEVICE_TOKEN, deviceToken)
            if (backendUrl.isNullOrBlank()) remove(KEY_BACKEND_URL)
            else putString(KEY_BACKEND_URL, backendUrl)
            putBoolean(KEY_IS_CHILD, isChild)
            apply()
        }
    }

    /// Reporta um evento. Só envia se for device de filho com token+URL
    /// configurados; caso contrário, no-op silencioso.
    fun report(context: Context, kind: String) {
        val prefs = prefs(context)
        val isChild = prefs.getBoolean(KEY_IS_CHILD, false)
        val token = prefs.getString(KEY_DEVICE_TOKEN, null)
        val baseUrl = prefs.getString(KEY_BACKEND_URL, null)
        if (!isChild || token.isNullOrBlank() || baseUrl.isNullOrBlank()) return

        thread(start = true) {
            try {
                val url = URL("${baseUrl.trimEnd('/')}/devices/tamper")
                val conn = (url.openConnection() as HttpURLConnection).apply {
                    requestMethod = "POST"
                    connectTimeout = TIMEOUT_MS
                    readTimeout = TIMEOUT_MS
                    doOutput = true
                    setRequestProperty("Content-Type", "application/json")
                }
                val body = JSONObject()
                    .put("device_token", token)
                    .put("kind", kind)
                    .toString()
                conn.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
                Log.i(TAG, "tamper '$kind' -> HTTP ${conn.responseCode}")
                conn.disconnect()
            } catch (e: Exception) {
                Log.w(TAG, "falha ao reportar tamper '$kind': ${e.message}")
            }
        }
    }
}
