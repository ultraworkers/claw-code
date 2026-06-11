package com.guardrail.plugin

import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.wm.StatusBar
import com.intellij.openapi.wm.StatusBarWidget
import com.intellij.openapi.wm.StatusBarWidgetFactory
import com.intellij.util.Consumer
import java.awt.Component
import java.awt.event.MouseEvent

class GuardrailStatusBarWidgetFactory : StatusBarWidgetFactory {
    override fun getId(): String = "GuardrailStatusBar"

    override fun getDisplayName(): String = "Guardrail"

    override fun isAvailable(project: Project): Boolean = true

    override fun createWidget(project: Project): StatusBarWidget {
        return GuardrailStatusBarWidget(project)
    }

    override fun disposeWidget(widget: StatusBarWidget) {
        Disposer.dispose(widget)
    }

    override fun canBeEnabledOn(statusBar: StatusBar): Boolean = true
}

class GuardrailStatusBarWidget(private val project: Project) : StatusBarWidget {
    private var statusBar: StatusBar? = null
    private var isConnected = false

    private val service: GuardrailService
        get() = project.getService(GuardrailService::class.java)

    override fun ID(): String = "GuardrailStatusBar"

    override fun getPresentation(): StatusBarWidget.WidgetPresentation {
        return object : StatusBarWidget.TextPresentation {
            override fun getTooltipText(): String {
                return when {
                    !service.isEnabled() -> "Guardrail is disabled"
                    isConnected -> "Guardrail connected - Click to configure"
                    else -> "Guardrail disconnected - Click to configure"
                }
            }

            override fun getText(): String {
                return when {
                    !service.isEnabled() -> "Guardrail: Off"
                    isConnected -> "Guardrail: Connected"
                    else -> "Guardrail: Disconnected"
                }
            }

            override fun getAlignment(): Float = Component.CENTER_ALIGNMENT

            override fun getClickConsumer(): Consumer<MouseEvent> {
                return Consumer { openConfiguration() }
            }
        }
    }

    override fun install(statusBar: StatusBar) {
        this.statusBar = statusBar
        testConnection()
    }

    override fun dispose() {
        statusBar = null
    }

    fun setConnected(connected: Boolean) {
        isConnected = connected
        statusBar?.updateWidget(ID())
    }

    fun update() {
        statusBar?.updateWidget(ID())
    }

    private fun openConfiguration() {
        com.intellij.openapi.options.ShowSettingsUtil.getInstance()
            .showSettingsDialog(project, GuardrailConfigurable::class.java)
    }

    private fun testConnection() {
        isConnected = service.testConnection()
        statusBar?.updateWidget(ID())
    }
}
