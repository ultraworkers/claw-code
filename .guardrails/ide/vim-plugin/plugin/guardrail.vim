" guardrail.vim - Guardrail validation for Vim
" Maintainer: TheArchitectit
" Version: 1.0.0

if exists('g:loaded_guardrail')
  finish
endif
let g:loaded_guardrail = 1

" Default configuration
let g:guardrail_server_url = get(g:, 'guardrail_server_url', 'http://localhost:8095')
let g:guardrail_api_key = get(g:, 'guardrail_api_key', '')
let g:guardrail_project_slug = get(g:, 'guardrail_project_slug', '')
let g:guardrail_enabled = get(g:, 'guardrail_enabled', 1)
let g:guardrail_validate_on_save = get(g:, 'guardrail_validate_on_save', 1)
let g:guardrail_severity_threshold = get(g:, 'guardrail_severity_threshold', 'warning')

" Commands
command! -nargs=0 GuardrailValidate call guardrail#ValidateBuffer()
command! -nargs=0 GuardrailValidateSelection call guardrail#ValidateSelection()
command! -nargs=0 GuardrailClear call guardrail#ClearDiagnostics()
command! -nargs=0 GuardrailTestConnection call guardrail#TestConnection()
command! -nargs=0 GuardrailConfig call guardrail#OpenConfig()

" Autocommands
if g:guardrail_enabled && g:guardrail_validate_on_save
  augroup Guardrail
    autocmd!
    autocmd BufWritePost * call guardrail#ValidateBuffer()
    autocmd BufUnload * call guardrail#ClearBufferDiagnostics(expand('<afile>'))
  augroup END
endif

" Statusline function
function! GuardrailStatus()
  return guardrail#Statusline()
endfunction
