package com.guardrail.plugin

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.openapi.diagnostic.Logger
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.util.concurrent.TimeUnit

data class GuardrailSettings(
    var serverUrl: String = "http://localhost:8095",
    // SECURITY NOTE: API key is stored in plain text in IDE settings.
    // For production use, migrate to PasswordSafe:
    // https://plugins.jetbrains.com/docs/intellij/persisting-sensitive-data.html
    var apiKey: String = "",
    var projectSlug: String = "",
    var enabled: Boolean = true,
    var validateOnSave: Boolean = true,
    var severityThreshold: String = "warning"
)

@Service(Service.Level.PROJECT)
@State(name = "GuardrailSettings", storages = [Storage("guardrail.xml")])
class GuardrailService : PersistentStateComponent<GuardrailSettings> {
    private val logger = Logger.getInstance(GuardrailService::class.java)
    private val gson = Gson()
    private var settings = GuardrailSettings()
    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()

    private val JSON_MEDIA_TYPE = "application/json; charset=utf-8".toMediaType()

    override fun getState(): GuardrailSettings = settings

    override fun loadState(state: GuardrailSettings) {
        settings = state
    }

    fun isEnabled(): Boolean = settings.enabled

    fun getSettings(): GuardrailSettings = settings

    fun updateSettings(newSettings: GuardrailSettings) {
        settings = newSettings
    }

    fun testConnection(): Boolean {
        return try {
            val request = buildRequest("/health/ready")
            client.newCall(request).execute().use { response ->
                response.isSuccessful
            }
        } catch (e: IOException) {
            // SECURITY: Log only error type, not message which may contain sensitive data
            logger.warn("Connection test failed: ${e.javaClass.simpleName}")
            false
        }
    }

    fun validateFile(filePath: String, content: String, language: String): ValidationResult {
        if (!settings.enabled) {
            return ValidationResult(true, emptyList())
        }

        val requestBody = gson.toJson(mapOf(
            "file_path" to filePath,
            "content" to content,
            "language" to language,
            "project_slug" to settings.projectSlug
        )).toRequestBody(JSON_MEDIA_TYPE)

        return executeValidation("/ide/validate/file", requestBody)
    }

    fun validateSelection(code: String, language: String): ValidationResult {
        if (!settings.enabled) {
            return ValidationResult(true, emptyList())
        }

        val requestBody = gson.toJson(mapOf(
            "code" to code,
            "language" to language
        )).toRequestBody(JSON_MEDIA_TYPE)

        return executeValidation("/ide/validate/selection", requestBody)
    }

    private fun executeValidation(endpoint: String, body: RequestBody): ValidationResult {
        return try {
            val request = buildRequest(endpoint, body)
            client.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    return ValidationResult(false, emptyList(), "HTTP ${response.code}")
                }

                response.body?.string()?.let { responseBody ->
                    val type = object : TypeToken<Map<String, Any>>() {}.type
                    val result = gson.fromJson<Map<String, Any>>(responseBody, type)

                    val valid = result["valid"] as? Boolean ?: false
                    @Suppress("UNCHECKED_CAST")
                    val violations = (result["violations"] as? List<Map<String, Any>>)?.map { v ->
                        Violation(
                            ruleId = v["rule_id"] as? String ?: "",
                            line = (v["line"] as? Number)?.toInt() ?: 1,
                            column = (v["column"] as? Number)?.toInt() ?: 1,
                            severity = v["severity"] as? String ?: "warning",
                            message = v["message"] as? String ?: "",
                            suggestion = v["suggestion"] as? String
                        )
                    } ?: emptyList()

                    ValidationResult(valid, violations)
                } ?: ValidationResult(false, emptyList(), "Empty response body")
            }
        } catch (e: IOException) {
            // SECURITY: Log only error type, not message which may contain sensitive data
            logger.error("Validation request failed: ${e.javaClass.simpleName}")
            ValidationResult(false, emptyList(), "Network error: connection failed")
        }
    }

    private fun buildRequest(path: String, body: RequestBody? = null): Request {
        // SECURITY: Validate server URL to prevent SSRF
        val serverUrl = settings.serverUrl.trim().removeSuffix("/")
        if (!serverUrl.startsWith("http://") && !serverUrl.startsWith("https://")) {
            throw IllegalArgumentException("Invalid server URL: must start with http:// or https://")
        }
        val url = "$serverUrl$path"
        val builder = Request.Builder().url(url)

        if (settings.apiKey.isNotEmpty()) {
            builder.header("Authorization", "Bearer ${settings.apiKey}")
        }

        builder.header("Content-Type", "application/json")

        return if (body != null) {
            builder.post(body).build()
        } else {
            builder.build()
        }
    }
}

data class ValidationResult(
    val valid: Boolean,
    val violations: List<Violation>,
    val error: String? = null
)

data class Violation(
    val ruleId: String,
    val line: Int,
    val column: Int,
    val severity: String,
    val message: String,
    val suggestion: String?
)
