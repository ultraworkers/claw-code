package com.guardrail.plugin

import com.intellij.openapi.options.Configurable
import com.intellij.openapi.project.Project
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.FormBuilder
import javax.swing.JCheckBox
import javax.swing.JComboBox
import javax.swing.JComponent
import javax.swing.JPanel
import javax.swing.JPasswordField

class GuardrailConfigurable(private val project: Project) : Configurable {
    private var panel: JPanel? = null
    private val serverUrlField = JBTextField()
    private val apiKeyField = JPasswordField()
    private val projectSlugField = JBTextField()
    private val enabledCheckBox = JCheckBox("Enable Guardrail")
    private val validateOnSaveCheckBox = JCheckBox("Validate on save")
    private val severityComboBox = JComboBox(arrayOf("info", "warning", "error"))

    private val service: GuardrailService
        get() = project.getService(GuardrailService::class.java)

    override fun getDisplayName(): String = "Guardrail"

    override fun createComponent(): JComponent {
        panel = FormBuilder.createFormBuilder()
            .addComponent(enabledCheckBox)
            .addLabeledComponent("Server URL:", serverUrlField)
            .addLabeledComponent("API Key:", apiKeyField)
            .addLabeledComponent("Project Slug:", projectSlugField)
            .addComponent(validateOnSaveCheckBox)
            .addLabeledComponent("Severity Threshold:", severityComboBox)
            .addComponentFillVertically(JPanel(), 0)
            .panel

        reset()
        return panel!!
    }

    override fun isModified(): Boolean {
        val settings = service.getSettings()
        return serverUrlField.text != settings.serverUrl ||
                apiKeyField.text != settings.apiKey ||
                projectSlugField.text != settings.projectSlug ||
                enabledCheckBox.isSelected != settings.enabled ||
                validateOnSaveCheckBox.isSelected != settings.validateOnSave ||
                severityComboBox.selectedItem != settings.severityThreshold
    }

    override fun apply() {
        val serverUrl = serverUrlField.text.trim()
        // SECURITY: Validate server URL to prevent SSRF
        if (!isValidServerUrl(serverUrl)) {
            throw com.intellij.openapi.options.ConfigurationException(
                "Invalid server URL. Must start with http:// or https://"
            )
        }

        val newSettings = GuardrailSettings(
            serverUrl = serverUrl,
            apiKey = apiKeyField.text,
            projectSlug = projectSlugField.text,
            enabled = enabledCheckBox.isSelected,
            validateOnSave = validateOnSaveCheckBox.isSelected,
            severityThreshold = severityComboBox.selectedItem as String
        )
        service.updateSettings(newSettings)
    }

    private fun isValidServerUrl(url: String): Boolean {
        return url.startsWith("http://") || url.startsWith("https://")
    }

    override fun reset() {
        val settings = service.getSettings()
        serverUrlField.text = settings.serverUrl
        apiKeyField.text = settings.apiKey
        projectSlugField.text = settings.projectSlug
        enabledCheckBox.isSelected = settings.enabled
        validateOnSaveCheckBox.isSelected = settings.validateOnSave
        severityComboBox.selectedItem = settings.severityThreshold
    }

    override fun getHelpTopic(): String? = null
}
