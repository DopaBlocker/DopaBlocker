package com.dopablocker.dopablocker_mobile.accessibility

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.Gravity
import android.view.ViewGroup
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

/// Tela full-screen exibida quando o filho abre um app bloqueado (C3) ou acessa
/// um site bloqueado no navegador (C1). O tipo vem em [EXTRA_KIND].
///
/// Técnica padrão (Cold Turkey/AppBlock): interpor uma Activity própria por
/// cima do app/navegador bloqueado é mais robusto do que só "trazer o DopaBlocker
/// pra frente" (que o usuário contorna voltando ao app). O layout é montado em
/// código para não depender de recursos de UI do Flutter.
class BlockOverlayActivity : Activity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val isSite = intent?.getStringExtra(EXTRA_KIND) == KIND_SITE
        val domain = intent?.getStringExtra(EXTRA_DOMAIN).orEmpty()

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setBackgroundColor(Color.parseColor("#101014"))
            setPadding(64, 64, 64, 64)
        }

        root.addView(TextView(this).apply {
            text = if (isSite) "Site bloqueado" else "App bloqueado"
            textSize = 26f
            setTextColor(Color.WHITE)
            gravity = Gravity.CENTER
        })

        root.addView(TextView(this).apply {
            text = when {
                isSite && domain.isNotEmpty() -> "$domain está bloqueado pelo DopaBlocker."
                isSite -> "Este site está bloqueado pelo DopaBlocker."
                else -> "Este aplicativo está bloqueado pelo DopaBlocker."
            }
            textSize = 15f
            setTextColor(Color.parseColor("#B0B0B8"))
            gravity = Gravity.CENTER
            setPadding(0, 24, 0, 32)
        })

        root.addView(Button(this).apply {
            text = "Voltar ao início"
            setOnClickListener { goHome() }
        })

        setContentView(
            root,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
    }

    // Voltar não retorna ao app bloqueado — manda para a home.
    @Deprecated("Comportamento intencional: redireciona para a home")
    override fun onBackPressed() {
        goHome()
    }

    private fun goHome() {
        startActivity(
            Intent(Intent.ACTION_MAIN).apply {
                addCategory(Intent.CATEGORY_HOME)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            },
        )
        finish()
    }

    companion object {
        const val EXTRA_PACKAGE = "blocked_package"

        /// Tipo de bloqueio exibido: app (padrão) ou site.
        const val EXTRA_KIND = "block_kind"

        /// Domínio bloqueado (quando [EXTRA_KIND] == [KIND_SITE]).
        const val EXTRA_DOMAIN = "blocked_domain"

        const val KIND_APP = "app"
        const val KIND_SITE = "site"
    }
}
