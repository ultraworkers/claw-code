package com.guardrail.plugin

import com.intellij.codeInspection.*
import com.intellij.codeInspection.util.InspectionMessage
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.editor.Document
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiDocumentManager
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile

class GuardrailInspection : LocalInspectionTool() {
    private val logger = Logger.getInstance(GuardrailInspection::class.java)

    override fun getDisplayName(): String = "Guardrail Validation"

    override fun getGroupDisplayName(): String = "Guardrail"

    override fun getStaticDescription(): String = "Validates code against Guardrail prevention rules"

    override fun isEnabledByDefault(): Boolean = true

    override fun runForWholeFile(): Boolean = true

    override fun checkFile(file: PsiFile, manager: InspectionManager, isOnTheFly: Boolean): Array<ProblemDescriptor>? {
        val project = file.project
        val service = project.getService(GuardrailService::class.java)

        if (!service.isEnabled()) {
            return null
        }

        val document = PsiDocumentManager.getInstance(project).getDocument(file) ?: return null
        val content = document.text
        val language = mapLanguage(file.language.id)
        val filePath = file.virtualFile?.path ?: return null

        val result = service.validateFile(filePath, content, language)

        if (result.error != null) {
            logger.warn("Validation error: ${result.error}")
            return null
        }

        val problems = mutableListOf<ProblemDescriptor>()

        for (violation in result.violations) {
            if (!shouldReport(violation.severity, service.getSettings().severityThreshold)) {
                continue
            }

            val lineStartOffset = document.getLineStartOffset(violation.line - 1)
            val lineEndOffset = document.getLineEndOffset(violation.line - 1)
            val startOffset = lineStartOffset + violation.column - 1
            val endOffset = minOf(startOffset + 1, lineEndOffset)

            val element = file.findElementAt(startOffset) ?: continue

            val fixes = createQuickFixes(violation)

            problems.add(
                manager.createProblemDescriptor(
                    element,
                    TextRange(startOffset - element.textRange.startOffset, endOffset - element.textRange.startOffset),
                    violation.message,
                    mapHighlightType(violation.severity),
                    isOnTheFly,
                    *fixes.toTypedArray()
                )
            )
        }

        return problems.toTypedArray()
    }

    private fun shouldReport(severity: String, threshold: String): Boolean {
        val levels = mapOf("info" to 1, "warning" to 2, "error" to 3)
        return levels[severity] ?: 0 >= levels[threshold] ?: 1
    }

    private fun mapHighlightType(severity: String): ProblemHighlightType {
        return when (severity) {
            "error" -> ProblemHighlightType.GENERIC_ERROR
            "warning" -> ProblemHighlightType.WEAK_WARNING
            else -> ProblemHighlightType.INFORMATION
        }
    }

    private fun mapLanguage(languageId: String): String {
        return when (languageId.lowercase()) {
            "javascript", "typescript" -> "javascript"
            "python" -> "python"
            "go" -> "go"
            "rust" -> "rust"
            "java" -> "java"
            "kotlin" -> "kotlin"
            "bash", "shell script" -> "bash"
            "yaml", "yml" -> "yaml"
            "json" -> "json"
            "markdown", "md" -> "markdown"
            "dockerfile" -> "dockerfile"
            else -> languageId.lowercase()
        }
    }

    private fun createQuickFixes(violation: Violation): List<LocalQuickFix> {
        val fixes = mutableListOf<LocalQuickFix>()

        violation.suggestion?.let { suggestion ->
            fixes.add(object : LocalQuickFix {
                override fun getFamilyName(): String = "Apply Guardrail suggestion"
                override fun getName(): String = suggestion

                override fun applyFix(project: Project, descriptor: ProblemDescriptor) {}
            })
        }

        return fixes
    }
}
