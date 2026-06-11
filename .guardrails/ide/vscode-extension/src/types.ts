export interface ValidationRequest {
    file_path: string;
    content: string;
    language: string;
    project_slug?: string;
}

export interface ValidationResponse {
    valid: boolean;
    violations: Violation[];
}

export interface Violation {
    rule_id: string;
    line: number;
    column: number;
    severity: 'error' | 'warning' | 'info';
    message: string;
    suggestion?: string;
    fix?: TextEdit;
}

export interface TextEdit {
    range: Range;
    new_text: string;
}

export interface Range {
    start: Position;
    end: Position;
}

export interface Position {
    line: number;
    column: number;
}

export interface GuardrailConfig {
    enabled: boolean;
    serverUrl: string;
    apiKey: string;
    projectSlug: string;
    validateOnSave: boolean;
    validateOnType: boolean;
    severityThreshold: 'error' | 'warning' | 'info';
}
