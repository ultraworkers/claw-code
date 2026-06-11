package com.guardrail.plugin

import com.intellij.lang.annotation.AnnotationHolder
import com.intellij.lang.annotation.ExternalAnnotator
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.project.Project
import com.intellij.psi.PsiFile

/**
 * External annotator that provides real-time guardrail validation.
 * This is registered in plugin.xml and called by the IDE's annotation pass.
 *
 * Note: doAnnotate() runs on a background thread by IntelliJ's annotation infrastructure.
 * See: https://plugins.jetbrains.com/docs/intellij/annotator.html
 */
class GuardrailAnnotator : ExternalAnnotator<GuardrailAnnotator.Info, List<Violation>>() {
    private val logger = Logger.getInstance(GuardrailAnnotator::class.java)

    data class Info(
        val project: Project,
        val filePath: String,
        val content: String,
        val language: String
    )

    override fun collectInformation(file: PsiFile): Info? {
        val virtualFile = file.virtualFile ?: return null
        val project = file.project
        return Info(
            project = project,
            filePath = virtualFile.path,
            content = file.text,
            language = file.language.id
        )
    }

    override fun doAnnotate(collectedInfo: Info?): List<Violation> {
        if (collectedInfo == null) return emptyList()

        // Get service from project - ExternalAnnotator runs on background thread
        val service = collectedInfo.project.getService(GuardrailService::class.java)
        if (!service.isEnabled()) return emptyList()

        return try {
            val result = service.validateFile(
                collectedInfo.filePath,
                collectedInfo.content,
                collectedInfo.language
            )
            result.violations
        } catch (e: Exception) {
            logger.warn("Guardrail validation failed", e)
            emptyList()
        }
    }

    override fun apply(file: PsiFile, violations: List<Violation>?, holder: AnnotationHolder) {
        violations?.forEach { violation ->
            // Find the element at the violation location
            val offset = getOffset(file, violation.line, violation.column)
            val element = file.findElementAt(offset)

            element?.let {
                val annotation = holder.newAnnotation(
                    when (violation.severity.lowercase()) {
                        "error" -> com.intellij.lang.annotation.HighlightSeverity.ERROR
                        "warning" -> com.intellij.lang.annotation.HighlightSeverity.WARNING
                        else -> com.intellij.lang.annotation.HighlightSeverity.WEAK_WARNING
                    },
                    violation.message
                )

                annotation.range(it.textRange)
                    .tooltip(buildTooltip(violation))
                    .needsUpdateOnTyping()

                violation.suggestion?.let { suggestion ->
                    annotation.withFix(GuardrailQuickFix(violation.ruleId, suggestion))
                }

                annotation.create()
            }
        }
    }

    private fun getOffset(file: PsiFile, line: Int, column: Int): Int {
        val document = com.intellij.openapi.editor.DocumentUtil.getDocument(file) ?: return 0
        val lineNumber = (line - 1).coerceIn(0, document.lineCount - 1)
        val lineStart = document.getLineStartOffset(lineNumber)
        val lineEnd = document.getLineEndOffset(lineNumber)
        return (lineStart + (column - 1)).coerceIn(lineStart, lineEnd)
    }

    private fun buildTooltip(violation: Violation): String {
        return buildString {
            append("<b>Guardrail Violation</b><br>")
            append(violation.message)
            if (violation.suggestion != null) {
                append("<br><br><b>Suggestion:</b> ")
                append(violation.suggestion)
            }
        }
    }
}

/**
 * Quick fix for guardrail violations.
 */
class GuardrailQuickFix(
    private val ruleId: String,
    private val suggestion: String
) : com.intellij.codeInsight.intention.IntentionAction {
    override fun getText(): String = "Apply Guardrail suggestion: $suggestion"
    override fun getFamilyName(): String = "Guardrail"
    override fun startInWriteAction(): Boolean = true

    override fun isAvailable(project: com.intellij.openapi.project.Project, editor: com.intellij.openapi.editor.Editor?, file: PsiFile?): Boolean = true

    override fun invoke(project: com.intellij.openapi.project.Project, editor: com.intellij.openapi.editor.Editor?, file: PsiFile?) {
        // Quick fix implementation would apply the suggestion
        // This is a simplified version - actual implementation would parse and apply the fix
        com.intellij.openapi.ui.Messages.showInfoMessage(
            "Suggestion for $ruleId: $suggestion",
            "Guardrail Quick Fix"
        )
    }
}

