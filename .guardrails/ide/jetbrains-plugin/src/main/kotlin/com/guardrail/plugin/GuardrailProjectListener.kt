package com.guardrail.plugin

import com.intellij.openapi.project.Project
import com.intellij.openapi.project.ProjectManagerListener

/**
 * Project lifecycle listener for Guardrail plugin.
 * Handles project open/close events and initializes/cleans up resources.
 */
class GuardrailProjectListener : ProjectManagerListener {

    override fun projectOpened(project: Project) {
        // Initialize project-specific resources when a project is opened
        val service = project.getService(GuardrailService::class.java)
        if (service != null && service.isEnabled()) {
            // Log that the plugin is active for this project
            com.intellij.openapi.diagnostic.Logger.getInstance(GuardrailProjectListener::class.java)
                .info("Guardrail plugin activated for project: ${project.name}")
        }
    }

    override fun projectClosing(project: Project) {
        // Clean up resources when a project is closing
        // This is called before the project is disposed
    }

    override fun projectClosed(project: Project) {
        // Final cleanup after project is closed
    }
}
