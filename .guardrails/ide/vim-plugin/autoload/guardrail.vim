" guardrail.vim - Core functionality for Guardrail validation
" Maintainer: TheArchitectit
" Version: 1.0.0

let s:curl_cmd = 'curl'
let s:namespace = 'guardrail'

" Initialize signs
if !hlexists('GuardrailError')
  highlight GuardrailError guifg=#ff0000 ctermfg=red
endif
if !hlexists('GuardrailWarning')
  highlight GuardrailWarning guifg=#ff9900 ctermfg=yellow
endif
if !hlexists('GuardrailInfo')
  highlight GuardrailInfo guifg=#0099ff ctermfg=blue
endif

sign define guardrail_error text=✗ texthl=GuardrailError
sign define guardrail_warning text=⚠ texthl=GuardrailWarning
sign define guardrail_info text=ℹ texthl=GuardrailInfo

" Configuration
function! s:GetConfig()
  return {
    \ 'server_url': g:guardrail_server_url,
    \ 'api_key': g:guardrail_api_key,
    \ 'project_slug': g:guardrail_project_slug,
    \ 'enabled': g:guardrail_enabled,
    \ 'severity_threshold': g:guardrail_severity_threshold
  \ }
endfunction

" Check if curl is available
function! s:CurlAvailable()
  if !executable(s:curl_cmd)
    echohl ErrorMsg
    echo 'Guardrail: curl is required but not found'
    echohl None
    return 0
  endif
  return 1
endfunction

" Test connection to MCP server
function! guardrail#TestConnection()
  if !s:CurlAvailable()
    return
  endif
  
  let l:config = s:GetConfig()
  let l:url = l:config.server_url . '/health/ready'
  let l:cmd = [s:curl_cmd, '-s', '-o', '/dev/null', '-w', '%{http_code}', shellescape(l:url)]

  if !empty(l:config.api_key)
    call add(l:cmd, '-H')
    call add(l:cmd, shellescape('Authorization: Bearer ' . l:config.api_key))
  endif

  let l:result = system(join(l:cmd, ' '))
  
  if l:result == '200'
    echo 'Guardrail: Connected'
  else
    echohl ErrorMsg
    echo 'Guardrail: Connection failed (HTTP ' . l:result . ')'
    echohl None
  endif
endfunction

" Validate current buffer
function! guardrail#ValidateBuffer()
  if !s:CurlAvailable()
    return
  endif
  
  let l:config = s:GetConfig()
  if !l:config.enabled
    return
  endif
  
  let l:filename = expand('%:p')
  if empty(l:filename)
    return
  endif
  
  let l:lines = getline(1, '$')
  let l:content = join(l:lines, "\n")
  let l:language = &filetype
  
  " Clear existing diagnostics
  call guardrail#ClearDiagnostics()
  
  " Build curl command
  let l:url = l:config.server_url . '/ide/validate/file'
  let l:body = json_encode({
    \ 'file_path': l:filename,
    \ 'content': l:content,
    \ 'language': l:language,
    \ 'project_slug': empty(l:config.project_slug) ? v:null : l:config.project_slug
  \ })
  
  let l:cmd = [s:curl_cmd, '-s', '-X', 'POST', shellescape(l:url)]
  call add(l:cmd, '-H')
  call add(l:cmd, shellescape('Content-Type: application/json'))

  if !empty(l:config.api_key)
    call add(l:cmd, '-H')
    call add(l:cmd, shellescape('Authorization: Bearer ' . l:config.api_key))
  endif

  call add(l:cmd, '-d')
  call add(l:cmd, shellescape(l:body))

  " Execute validation
  let l:result = system(join(l:cmd, ' '))
  
  if v:shell_error != 0
    echohl ErrorMsg
    echo 'Guardrail: Validation request failed'
    echohl None
    return
  endif
  
  try
    let l:response = json_decode(l:result)
    call s:ProcessResponse(l:response)
  catch
    echohl ErrorMsg
    echo 'Guardrail: Failed to parse response'
    echohl None
  endtry
endfunction

" Validate visual selection
function! guardrail#ValidateSelection()
  if !s:CurlAvailable()
    return
  endif
  
  let l:config = s:GetConfig()
  if !l:config.enabled
    return
  endif
  
  " Get visual selection
  let [l:line1, l:col1] = getpos("'<")[1:2]
  let [l:line2, l:col2] = getpos("'>")[1:2]
  
  if l:line1 == 0 || l:line2 == 0
    echo 'Guardrail: No selection'
    return
  endif
  
  let l:lines = getline(l:line1, l:line2)
  if len(l:lines) == 0
    echo 'Guardrail: No selection'
    return
  endif
  
  let l:code = join(l:lines, "\n")
  let l:language = &filetype
  
  " Build curl command
  let l:url = l:config.server_url . '/ide/validate/selection'
  let l:body = json_encode({
    \ 'code': l:code,
    \ 'language': l:language
  \ })
  
  let l:cmd = [s:curl_cmd, '-s', '-X', 'POST', shellescape(l:url)]
  call add(l:cmd, '-H')
  call add(l:cmd, shellescape('Content-Type: application/json'))

  if !empty(l:config.api_key)
    call add(l:cmd, '-H')
    call add(l:cmd, shellescape('Authorization: Bearer ' . l:config.api_key))
  endif

  call add(l:cmd, '-d')
  call add(l:cmd, shellescape(l:body))

  let l:result = system(join(l:cmd, ' '))
  
  if v:shell_error != 0
    echohl ErrorMsg
    echo 'Guardrail: Validation request failed'
    echohl None
    return
  endif
  
  try
    let l:response = json_decode(l:result)
    call s:ProcessSelectionResponse(l:response)
  catch
    echohl ErrorMsg
    echo 'Guardrail: Failed to parse response'
    echohl None
  endtry
endfunction

" Process validation response
function! s:ProcessResponse(response)
  if !has_key(a:response, 'violations')
    return
  endif
  
  let l:violations = a:response.violations
  let l:count = len(l:violations)
  
  if l:count == 0
    echo 'Guardrail: No violations found'
    return
  endif
  
  let l:config = s:GetConfig()
  let l:reported = 0
  
  for l:violation in l:violations
    if s:ShouldReport(l:violation.severity, l:config.severity_threshold)
      call s:PlaceSign(l:violation)
      call s:AddLocation(l:violation)
      let l:reported += 1
    endif
  endfor
  
  if l:reported > 0
    echo 'Guardrail: Found ' . l:reported . ' violation(s)'
  else
    echo 'Guardrail: No violations above threshold'
  endif
endfunction

" Process selection validation response
function! s:ProcessSelectionResponse(response)
  if !has_key(a:response, 'violations')
    return
  endif
  
  let l:violations = a:response.violations
  let l:count = len(l:violations)
  
  if l:count == 0
    echo 'Guardrail: Selection is valid'
    return
  endif
  
  let l:messages = []
  for l:violation in l:violations
    call add(l:messages, '- ' . l:violation.message)
  endfor
  
  echohl WarningMsg
  echo 'Guardrail: Found ' . l:count . " violation(s):\n" . join(l:messages, "\n")
  echohl None
endfunction

" Check if severity should be reported
function! s:ShouldReport(severity, threshold)
  let l:levels = {'info': 1, 'warning': 2, 'error': 3}
  let l:sev_level = get(l:levels, a:severity, 0)
  let l:thresh_level = get(l:levels, a:threshold, 1)
  return l:sev_level >= l:thresh_level
endfunction

" Place sign for violation
function! s:PlaceSign(violation)
  let l:line = a:violation.line
  let l:severity = a:violation.severity
  
  if l:severity == 'error'
    let l:sign = 'guardrail_error'
  elseif l:severity == 'warning'
    let l:sign = 'guardrail_warning'
  else
    let l:sign = 'guardrail_info'
  endif
  
  execute 'sign place ' . line('.') . ' line=' . l:line . ' name=' . l:sign . ' buffer=' . bufnr('%')
endfunction

" Add to location list
function! s:AddLocation(violation)
  let l:item = {
    \ 'bufnr': bufnr('%'),
    \ 'lnum': a:violation.line,
    \ 'col': a:violation.column,
    \ 'text': a:violation.message,
    \ 'type': a:violation.severity == 'error' ? 'E' : a:violation.severity == 'warning' ? 'W' : 'I'
  \ }
  
  call setloclist(0, [l:item], 'a')
endfunction

" Clear diagnostics for current buffer
function! guardrail#ClearDiagnostics()
  execute 'sign unplace * buffer=' . bufnr('%')
  call setloclist(0, [], 'r')
endfunction

" Clear diagnostics for specific buffer
function! guardrail#ClearBufferDiagnostics(filename)
  let l:bufnr = bufnr(a:filename)
  if l:bufnr != -1
    execute 'sign unplace * buffer=' . l:bufnr
  endif
endfunction

" Statusline component
function! guardrail#Statusline()
  let l:config = s:GetConfig()
  
  if !l:config.enabled
    return 'Guardrail: Off'
  endif
  
  return 'Guardrail: On'
endfunction

" Open configuration
function! guardrail#OpenConfig()
  " Open vimrc or init.vim
  if has('nvim')
    edit $MYVIMRC
  else
    edit $VIMRC
  endif
  
  " Move to end and add template
  normal! G
  call append(line('$'), [
    \ '',
    \ '" Guardrail Configuration',
    \ 'let g:guardrail_server_url = "http://localhost:8095"',
    \ 'let g:guardrail_api_key = ""',
    \ 'let g:guardrail_project_slug = ""',
    \ 'let g:guardrail_enabled = 1',
    \ 'let g:guardrail_validate_on_save = 1',
    \ 'let g:guardrail_severity_threshold = "warning"'
  \ ])
endfunction
