from __future__ import annotations

import io
import json
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from src.agent_loop import (
    AgentRunResult,
    build_agent_messages,
    has_successful_edit,
    infer_tool_call_from_prompt,
    infer_required_edit_target,
    parse_agent_response,
    prompt_requires_edit,
    prompt_requires_tool,
    run_agent_task,
    tool_result_message,
)
from src.commands import PORTED_COMMANDS
from src.llm_backend import normalize_ollama_response
from src.parity_audit import run_parity_audit
from src.port_manifest import build_port_manifest
from src.query_engine import QueryEnginePort
from src.runtime_tools import RuntimeToolRegistry
from src.task import DEFAULT_SYSTEM_PROMPT, build_chat_messages, run_local_task
from src.task_packet import TaskPacket, TaskPacketValidationError, load_task_packet
from src.tools import PORTED_TOOLS
from src.worker_api import close_worker, create_worker, list_workers, load_worker, resume_worker, run_worker


class PortingWorkspaceTests(unittest.TestCase):
    def test_manifest_counts_python_files(self) -> None:
        manifest = build_port_manifest()
        self.assertGreaterEqual(manifest.total_python_files, 20)
        self.assertTrue(manifest.top_level_modules)

    def test_query_engine_summary_mentions_workspace(self) -> None:
        summary = QueryEnginePort.from_workspace().render_summary()
        self.assertIn('Python Porting Workspace Summary', summary)
        self.assertIn('Command surface:', summary)
        self.assertIn('Tool surface:', summary)

    def test_cli_summary_runs(self) -> None:
        result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'summary'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('Python Porting Workspace Summary', result.stdout)

    def test_parity_audit_runs(self) -> None:
        result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'parity-audit'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('Parity Audit', result.stdout)

    def test_root_file_coverage_is_complete_when_local_archive_exists(self) -> None:
        audit = run_parity_audit()
        if audit.archive_present:
            self.assertEqual(audit.root_file_coverage[0], audit.root_file_coverage[1])
            self.assertGreaterEqual(audit.directory_coverage[0], 28)
            self.assertGreaterEqual(audit.command_entry_ratio[0], 150)
            self.assertGreaterEqual(audit.tool_entry_ratio[0], 100)

    def test_command_and_tool_snapshots_are_nontrivial(self) -> None:
        self.assertGreaterEqual(len(PORTED_COMMANDS), 150)
        self.assertGreaterEqual(len(PORTED_TOOLS), 100)

    def test_commands_and_tools_cli_run(self) -> None:
        commands_result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'commands', '--limit', '5', '--query', 'review'],
            check=True,
            capture_output=True,
            text=True,
        )
        tools_result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'tools', '--limit', '5', '--query', 'MCP'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('Command entries:', commands_result.stdout)
        self.assertIn('Tool entries:', tools_result.stdout)

    def test_subsystem_packages_expose_archive_metadata(self) -> None:
        from src import assistant, bridge, utils

        self.assertGreater(assistant.MODULE_COUNT, 0)
        self.assertGreater(bridge.MODULE_COUNT, 0)
        self.assertGreater(utils.MODULE_COUNT, 100)
        self.assertTrue(utils.SAMPLE_FILES)

    def test_route_and_show_entry_cli_run(self) -> None:
        route_result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'route', 'review MCP tool', '--limit', '5'],
            check=True,
            capture_output=True,
            text=True,
        )
        show_command = subprocess.run(
            [sys.executable, '-m', 'src.main', 'show-command', 'review'],
            check=True,
            capture_output=True,
            text=True,
        )
        show_tool = subprocess.run(
            [sys.executable, '-m', 'src.main', 'show-tool', 'MCPTool'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('review', route_result.stdout.lower())
        self.assertIn('review', show_command.stdout.lower())
        self.assertIn('mcptool', show_tool.stdout.lower())

    def test_bootstrap_cli_runs(self) -> None:
        result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'bootstrap', 'review MCP tool', '--limit', '5'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('Runtime Session', result.stdout)
        self.assertIn('Startup Steps', result.stdout)
        self.assertIn('Routed Matches', result.stdout)

    def test_bootstrap_session_tracks_turn_state(self) -> None:
        from src.runtime import PortRuntime

        session = PortRuntime().bootstrap_session('review MCP tool', limit=5)
        self.assertGreaterEqual(len(session.turn_result.matched_tools), 1)
        self.assertIn('Prompt:', session.turn_result.output)
        self.assertGreaterEqual(session.turn_result.usage.input_tokens, 1)

    def test_exec_command_and_tool_cli_run(self) -> None:
        command_result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'exec-command', 'review', 'inspect security review'],
            check=True,
            capture_output=True,
            text=True,
        )
        tool_result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'exec-tool', 'MCPTool', 'fetch resource list'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn("Mirrored command 'review'", command_result.stdout)
        self.assertIn("Mirrored tool 'MCPTool'", tool_result.stdout)

    def test_setup_report_and_registry_filters_run(self) -> None:
        setup_result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'setup-report'],
            check=True,
            capture_output=True,
            text=True,
        )
        command_result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'commands', '--limit', '5', '--no-plugin-commands'],
            check=True,
            capture_output=True,
            text=True,
        )
        tool_result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'tools', '--limit', '5', '--simple-mode', '--no-mcp'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('Setup Report', setup_result.stdout)
        self.assertIn('Command entries:', command_result.stdout)
        self.assertIn('Tool entries:', tool_result.stdout)

    def test_plugin_command_filter_excludes_plugin_sources(self) -> None:
        from src.commands import get_commands

        all_commands = get_commands()
        filtered_commands = get_commands(include_plugin_commands=False)

        self.assertGreater(len(all_commands), len(filtered_commands))
        self.assertFalse(
            any('plugin' in command.source_hint.lower() for command in filtered_commands)
        )

    def test_plugin_command_aliases_execute_as_local_commands(self) -> None:
        for alias in ('plugin', 'plugins', 'marketplace'):
            with self.subTest(alias=alias):
                result = subprocess.run(
                    [sys.executable, '-m', 'src.main', 'exec-command', alias, f'{alias} list'],
                    check=True,
                    capture_output=True,
                    text=True,
                )

                self.assertIn("Mirrored command 'plugin'", result.stdout)
                self.assertNotIn('Unknown mirrored command', result.stdout)

    def test_route_plugin_slash_commands_match_commands(self) -> None:
        prompts = ('/plugin list', '/plugins list', '/marketplace browse', '/reload-plugins')
        for prompt in prompts:
            with self.subTest(prompt=prompt):
                result = subprocess.run(
                    [sys.executable, '-m', 'src.main', 'route', prompt, '--limit', '5'],
                    check=True,
                    capture_output=True,
                    text=True,
                )

                first_line = result.stdout.splitlines()[0]
                self.assertTrue(first_line.startswith('command\t'), result.stdout)
                self.assertRegex(first_line, r'\t(plugin|reload-plugins)\t')

    def test_plugin_command_stream_emits_command_match(self) -> None:
        from src.runtime import PortRuntime

        for prompt in ('/plugin list', '/plugins list', '/marketplace browse', '/reload-plugins'):
            with self.subTest(prompt=prompt):
                session = PortRuntime().bootstrap_session(prompt, limit=5)
                command_events = [
                    event for event in session.stream_events if event['type'] == 'command_match'
                ]

                self.assertTrue(command_events, session.as_markdown())
                self.assertNotIn('Matched commands: none', session.turn_result.output)

    def test_turn_loop_plugin_commands_are_not_prompt_only(self) -> None:
        for prompt in ('/plugin list', '/plugins list', '/marketplace browse', '/reload-plugins'):
            with self.subTest(prompt=prompt):
                result = subprocess.run(
                    [
                        sys.executable,
                        '-m',
                        'src.main',
                        'turn-loop',
                        prompt,
                        '--max-turns',
                        '1',
                        '--structured-output',
                    ],
                    check=True,
                    capture_output=True,
                    text=True,
                )

                self.assertIn('"Matched commands:', result.stdout)
                self.assertNotIn('Matched commands: none', result.stdout)

    def test_load_session_cli_runs(self) -> None:
        from src.runtime import PortRuntime

        session = PortRuntime().bootstrap_session('review MCP tool', limit=5)
        session_id = Path(session.persisted_session_path).stem
        result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'load-session', session_id],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn(session_id, result.stdout)
        self.assertIn('messages', result.stdout)

    def test_tool_permission_filtering_cli_runs(self) -> None:
        result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'tools', '--limit', '10', '--deny-prefix', 'mcp'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('Tool entries:', result.stdout)
        self.assertNotIn('MCPTool', result.stdout)

    def test_turn_loop_cli_runs(self) -> None:
        result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'turn-loop', 'review MCP tool', '--max-turns', '2', '--structured-output'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('## Turn 1', result.stdout)
        self.assertIn('stop_reason=', result.stdout)

    def test_remote_mode_clis_run(self) -> None:
        remote_result = subprocess.run([sys.executable, '-m', 'src.main', 'remote-mode', 'workspace'], check=True, capture_output=True, text=True)
        ssh_result = subprocess.run([sys.executable, '-m', 'src.main', 'ssh-mode', 'workspace'], check=True, capture_output=True, text=True)
        teleport_result = subprocess.run([sys.executable, '-m', 'src.main', 'teleport-mode', 'workspace'], check=True, capture_output=True, text=True)
        self.assertIn('mode=remote', remote_result.stdout)
        self.assertIn('mode=ssh', ssh_result.stdout)
        self.assertIn('mode=teleport', teleport_result.stdout)

    def test_flush_transcript_cli_runs(self) -> None:
        result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'flush-transcript', 'review MCP tool'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('flushed=True', result.stdout)

    def test_command_graph_and_tool_pool_cli_run(self) -> None:
        command_graph = subprocess.run([sys.executable, '-m', 'src.main', 'command-graph'], check=True, capture_output=True, text=True)
        tool_pool = subprocess.run([sys.executable, '-m', 'src.main', 'tool-pool'], check=True, capture_output=True, text=True)
        self.assertIn('Command Graph', command_graph.stdout)
        self.assertIn('Tool Pool', tool_pool.stdout)

    def test_setup_report_mentions_deferred_init(self) -> None:
        result = subprocess.run(
            [sys.executable, '-m', 'src.main', 'setup-report'],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn('Deferred init:', result.stdout)
        self.assertIn('plugin_init=True', result.stdout)

    def test_execution_registry_runs(self) -> None:
        from src.execution_registry import build_execution_registry

        registry = build_execution_registry()
        self.assertGreaterEqual(len(registry.commands), 150)
        self.assertGreaterEqual(len(registry.tools), 100)
        self.assertIn('Mirrored command', registry.command('review').execute('review security'))
        self.assertIn('Mirrored tool', registry.tool('MCPTool').execute('fetch mcp resources'))

    def test_bootstrap_graph_and_direct_modes_run(self) -> None:
        graph_result = subprocess.run([sys.executable, '-m', 'src.main', 'bootstrap-graph'], check=True, capture_output=True, text=True)
        direct_result = subprocess.run([sys.executable, '-m', 'src.main', 'direct-connect-mode', 'workspace'], check=True, capture_output=True, text=True)
        deep_link_result = subprocess.run([sys.executable, '-m', 'src.main', 'deep-link-mode', 'workspace'], check=True, capture_output=True, text=True)
        self.assertIn('Bootstrap Graph', graph_result.stdout)
        self.assertIn('mode=direct-connect', direct_result.stdout)
        self.assertIn('mode=deep-link', deep_link_result.stdout)

    def test_build_chat_messages_includes_default_system_prompt(self) -> None:
        messages = build_chat_messages('Summarize this repo')
        self.assertEqual(messages[0], {'role': 'system', 'content': DEFAULT_SYSTEM_PROMPT})
        self.assertEqual(messages[1], {'role': 'user', 'content': 'Summarize this repo'})

    def test_run_local_task_uses_backend_response_content(self) -> None:
        with patch('src.task.OllamaBackend.chat', return_value=normalize_ollama_response({'message': {'content': 'local answer'}})) as chat_mock:
            result = run_local_task('hello')
        self.assertEqual(result, 'local answer')
        sent_messages = chat_mock.call_args.args[0]
        self.assertEqual(sent_messages[-1], {'role': 'user', 'content': 'hello'})

    def test_normalize_ollama_response_returns_assistant_content(self) -> None:
        response = normalize_ollama_response(
            {
                'model': 'qwen2.5-coder:7b',
                'message': {'role': 'assistant', 'content': 'hello from ollama'},
            }
        )
        self.assertEqual(response.content, 'hello from ollama')
        self.assertEqual(response.raw['model'], 'qwen2.5-coder:7b')

    def test_chat_cli_reads_prompt_from_stdin(self) -> None:
        from src.main import build_parser, resolve_chat_prompt

        parser = build_parser()
        args = parser.parse_args(['chat'])
        with patch('sys.stdin.read', return_value='stdin prompt'):
            self.assertEqual(resolve_chat_prompt(args, parser), 'stdin prompt')

    def test_runtime_tool_registry_reads_searches_edits_and_runs_shell(self) -> None:
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            target = root / 'notes.txt'
            target.write_text('alpha\nbeta\n', encoding='utf-8')
            registry = RuntimeToolRegistry(root=root)

            read_result = registry.execute('read_file', {'path': 'notes.txt'})
            self.assertTrue(read_result.success)
            self.assertIn('1\talpha', read_result.output)

            search_result = registry.execute('search_files', {'pattern': 'beta', 'path': '.'})
            self.assertTrue(search_result.success)
            self.assertIn('notes.txt:2:beta', search_result.output)

            edit_result = registry.execute(
                'edit_file',
                {'path': 'notes.txt', 'old_text': 'beta\n', 'new_text': 'gamma\n'},
            )
            self.assertTrue(edit_result.success)
            self.assertEqual(target.read_text(encoding='utf-8'), 'alpha\ngamma\n')

            shell_result = registry.execute('run_shell_command', {'command': 'printf shell-ok'})
            self.assertTrue(shell_result.success)
            self.assertEqual(shell_result.output, 'shell-ok')

    def test_runtime_tool_registry_supports_list_dir_git_build_and_test(self) -> None:
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / 'src').mkdir()
            (root / 'src' / 'app.py').write_text('print("ok")\n', encoding='utf-8')
            subprocess.run(['git', 'init'], cwd=root, check=True, capture_output=True, text=True)
            subprocess.run(['git', 'config', 'user.email', 'test@example.com'], cwd=root, check=True, capture_output=True, text=True)
            subprocess.run(['git', 'config', 'user.name', 'Test User'], cwd=root, check=True, capture_output=True, text=True)
            subprocess.run(['git', 'add', '.'], cwd=root, check=True, capture_output=True, text=True)
            subprocess.run(['git', 'commit', '-m', 'init'], cwd=root, check=True, capture_output=True, text=True)
            (root / 'src' / 'app.py').write_text('print("changed")\n', encoding='utf-8')
            registry = RuntimeToolRegistry(root=root)

            list_result = registry.execute('list_dir', {'path': 'src'})
            self.assertTrue(list_result.success)
            self.assertIn('[F] src/app.py', list_result.output)

            git_status = registry.execute('git_status', {})
            self.assertTrue(git_status.success)
            self.assertIn('src/app.py', git_status.output)

            git_diff = registry.execute('git_diff', {'path': 'src/app.py'})
            self.assertTrue(git_diff.success)
            self.assertIn('-print("ok")', git_diff.output)

            run_build = registry.execute('run_build', {'command': 'python3 -m compileall src'})
            self.assertTrue(run_build.success)

            run_tests = registry.execute('run_tests', {'command': 'python3 -c "print(\'tests ok\')"', 'timeout_seconds': 30})
            self.assertTrue(run_tests.success)
            self.assertIn('tests ok', run_tests.output)

    def test_parse_agent_response_supports_tool_calls_and_finals(self) -> None:
        tool_call = parse_agent_response('{"type":"tool_call","name":"read_file","arguments":{"path":"src/main.py"}}')
        final = parse_agent_response('{"type":"final","content":"done"}')
        self.assertEqual(tool_call['name'], 'read_file')
        self.assertEqual(final['content'], 'done')

    def test_prompt_requires_tool_for_workspace_inspection_prompts(self) -> None:
        self.assertTrue(prompt_requires_tool('read src/main.py lines 1 to 40'))
        self.assertTrue(prompt_requires_tool("search the src tree for 'agent_parser'"))
        self.assertFalse(prompt_requires_tool('say hello'))

    def test_prompt_requires_edit_for_mutating_prompts(self) -> None:
        self.assertTrue(prompt_requires_edit('modify train.py by adding a single harmless comment line near the top of the file'))
        self.assertFalse(prompt_requires_edit('read train.py for context'))
        self.assertEqual(
            infer_required_edit_target('modify train.py by adding a single harmless comment line near the top of the file'),
            'train.py',
        )

    def test_infer_required_edit_target_prefers_unique_file_reference(self) -> None:
        prompt = (
            'Objective: In train.py, Change WEIGHT_DECAY from 1e-4 to 5e-5. '
            'Scope: Modify only train.py in the autoresearch repo. '
            'You may read program.md and README.md but do not edit prepare.py or pyproject.toml.'
        )
        self.assertEqual(infer_required_edit_target(prompt), 'train.py')

    def test_infer_tool_call_from_prompt_recognizes_read_and_search(self) -> None:
        read_call = infer_tool_call_from_prompt('read src/main.py lines 1 to 40')
        search_call = infer_tool_call_from_prompt("search the src tree for 'agent_parser'")
        self.assertEqual(read_call, {'name': 'read_file', 'arguments': {'path': 'src/main.py', 'start_line': 1, 'end_line': 40}})
        self.assertEqual(search_call, {'name': 'search_files', 'arguments': {'path': 'src', 'pattern': 'agent_parser'}})

    def test_infer_tool_call_from_prompt_recognizes_comment_edit_fallback(self) -> None:
        tool_call = infer_tool_call_from_prompt(
            'modify train.py by adding a single harmless comment line near the top of the file',
            prefer_edit=True,
        )
        self.assertIsNotNone(tool_call)
        self.assertEqual(tool_call['name'], 'run_shell_command')
        self.assertIn('target = Path("train.py")', tool_call['arguments']['command'])
        self.assertIn('# smoke-test comment: ', tool_call['arguments']['command'])

    def test_infer_tool_call_from_prompt_recognizes_incrementing_comment_prompt(self) -> None:
        tool_call = infer_tool_call_from_prompt(
            'Modify train.py by incrementing or inserting a harmless smoke-test comment counter near the top of the file. You must change train.py on every run. A no-op is failure.',
            prefer_edit=True,
        )
        self.assertIsNotNone(tool_call)
        self.assertEqual(tool_call['name'], 'run_shell_command')
        self.assertIn('target = Path("train.py")', tool_call['arguments']['command'])
        self.assertIn('# smoke-test comment: ', tool_call['arguments']['command'])

    def test_tool_result_message_is_json_prefixed(self) -> None:
        message = tool_result_message({'name': 'read_file', 'success': True, 'output': 'ok'})
        self.assertTrue(message.startswith('TOOL_RESULT '))
        self.assertIn('"name": "read_file"', message)

    def test_build_agent_messages_mentions_workspace_and_tools(self) -> None:
        messages = build_agent_messages('inspect repo', Path('/tmp/workspace'))
        self.assertEqual(messages[1], {'role': 'user', 'content': 'inspect repo'})
        self.assertIn('Workspace root: /tmp/workspace', messages[0]['content'])
        self.assertIn('read_file', messages[0]['content'])

    def test_run_agent_task_executes_tools_end_to_end(self) -> None:
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / 'example.txt').write_text('hello world\n', encoding='utf-8')
            responses = [
                normalize_ollama_response(
                    {
                        'message': {
                            'content': '{"type":"tool_call","name":"read_file","arguments":{"path":"example.txt"}}'
                        }
                    }
                ),
                normalize_ollama_response(
                    {
                        'message': {
                            'content': '{"type":"final","content":"Read complete."}'
                        }
                    }
                ),
            ]
            with patch('src.agent_loop.OllamaBackend.chat', side_effect=responses) as chat_mock:
                result = run_agent_task('inspect file', root=root, max_turns=3, trace=True)

            self.assertEqual(result.content, 'Read complete.')
            self.assertEqual(result.tool_calls, ('read_file',))
            self.assertEqual(result.stop_reason, 'completed')
            self.assertTrue(any(event.startswith('tool_call') for event in result.trace_events))
            self.assertEqual(result.tool_trace[0]['name'], 'read_file')
            second_call_messages = chat_mock.call_args_list[1].args[0]
            self.assertTrue(any(message['content'].startswith('TOOL_RESULT ') for message in second_call_messages if message['role'] == 'user'))

    def test_run_agent_task_rejects_direct_final_when_tool_is_required(self) -> None:
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / 'example.txt').write_text('hello world\n', encoding='utf-8')
            responses = [
                normalize_ollama_response(
                    {
                        'message': {
                            'content': '{"type":"final","content":"No subcommands found."}'
                        }
                    }
                ),
                normalize_ollama_response(
                    {
                        'message': {
                            'content': '{"type":"final","content":"Read complete after tool use."}'
                        }
                    }
                ),
            ]
            with patch('src.agent_loop.OllamaBackend.chat', side_effect=responses):
                result = run_agent_task('read example.txt lines 1 to 1', root=root, max_turns=3, trace=True)

            self.assertEqual(result.content, 'Read complete after tool use.')
            self.assertEqual(result.tool_calls, ('read_file',))
            self.assertTrue(any('tool_required_reject' in event for event in result.trace_events))

    def test_run_agent_task_requires_mutation_for_edit_prompt(self) -> None:
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            target = root / 'train.py'
            target.write_text('print("ok")\n', encoding='utf-8')
            responses = [
                normalize_ollama_response(
                    {
                        'message': {
                            'content': '{"type":"tool_call","name":"read_file","arguments":{"path":"train.py"}}'
                        }
                    }
                ),
                normalize_ollama_response(
                    {
                        'message': {
                            'content': '{"type":"final","content":"I reviewed the file."}'
                        }
                    }
                ),
                normalize_ollama_response(
                    {
                        'message': {
                            'content': '{"type":"final","content":"Inserted the comment."}'
                        }
                    }
                ),
            ]
            with patch('src.agent_loop.OllamaBackend.chat', side_effect=responses):
                result = run_agent_task(
                    'modify train.py by adding a single harmless comment line near the top of the file',
                    root=root,
                    max_turns=4,
                    trace=True,
                )

            self.assertIn('# smoke-test comment: 1', target.read_text(encoding='utf-8'))
            self.assertEqual(result.tool_calls, ('read_file', 'run_shell_command'))
            self.assertTrue(any('edit_required_reject' in event for event in result.trace_events))

    def test_run_agent_task_incrementing_smoke_comment_is_repeatable(self) -> None:
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            target = root / 'train.py'
            target.write_text('# smoke-test comment: 1\nprint("ok")\n', encoding='utf-8')
            responses = [
                normalize_ollama_response(
                    {
                        'message': {
                            'content': '{"type":"tool_call","name":"read_file","arguments":{"path":"train.py"}}'
                        }
                    }
                ),
                normalize_ollama_response(
                    {
                        'message': {
                            'content': '{"type":"final","content":"I reviewed the file."}'
                        }
                    }
                ),
                normalize_ollama_response(
                    {
                        'message': {
                            'content': '{"type":"final","content":"Updated the smoke comment."}'
                        }
                    }
                ),
            ]
            with patch('src.agent_loop.OllamaBackend.chat', side_effect=responses):
                run_agent_task(
                    'modify train.py by adding a single harmless comment line near the top of the file',
                    root=root,
                    max_turns=4,
                    trace=True,
                )

            self.assertTrue(target.read_text(encoding='utf-8').startswith('# smoke-test comment: 2\n'))

    def test_run_agent_task_accepts_successful_edit_at_max_turns(self) -> None:
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            target = root / 'train.py'
            target.write_text('WEIGHT_DECAY = 1e-4\n', encoding='utf-8')
            responses = [
                normalize_ollama_response(
                    {
                        'message': {
                            'content': (
                                '{"type":"tool_call","name":"edit_file","arguments":'
                                '{"path":"train.py","old_text":"WEIGHT_DECAY = 1e-4",'
                                '"new_text":"WEIGHT_DECAY = 5e-5"}}'
                            )
                        }
                    }
                ),
            ]
            with patch('src.agent_loop.OllamaBackend.chat', side_effect=responses):
                result = run_agent_task(
                    'Objective: In train.py, Change WEIGHT_DECAY from 1e-4 to 5e-5.',
                    root=root,
                    max_turns=1,
                    trace=True,
                )

            self.assertEqual(target.read_text(encoding='utf-8'), 'WEIGHT_DECAY = 5e-5\n')
            self.assertEqual(result.stop_reason, 'completed_after_edit_max_turns')
            self.assertEqual(result.tool_calls, ('edit_file',))
            self.assertTrue(any('stopped=max_turns:1' in event for event in result.trace_events))

    def test_has_successful_edit_requires_target_path(self) -> None:
        tool_trace = [
            {
                'name': 'edit_file',
                'success': False,
                'arguments': {'path': 'program.md'},
                'metadata': {'path': 'program.md'},
            },
            {
                'name': 'run_shell_command',
                'success': True,
                'arguments': {'command': 'echo ok'},
                'metadata': {'path': 'program.md'},
            },
        ]
        self.assertFalse(has_successful_edit(tool_trace, 'train.py'))
        self.assertTrue(has_successful_edit(tool_trace, 'program.md'))

    def test_task_packet_load_and_validation(self) -> None:
        with TemporaryDirectory() as tmpdir:
            packet_path = Path(tmpdir) / 'task.json'
            packet_path.write_text(
                (
                    '{'
                    '"objective":"Update docs",'
                    '"scope":"README only",'
                    '"repo":"claw-code",'
                    '"branch_policy":"stay on current branch",'
                    '"acceptance_tests":["python3 -m unittest discover -s tests -v"],'
                    '"commit_policy":"no commit",'
                    '"reporting_contract":"report changed files",'
                    '"escalation_policy":"stop on ambiguity"'
                    '}'
                ),
                encoding='utf-8',
            )
            packet = load_task_packet(packet_path)
            self.assertEqual(packet.objective, 'Update docs')
            self.assertEqual(packet.acceptance_tests, ('python3 -m unittest discover -s tests -v',))

    def test_task_packet_rejects_missing_fields(self) -> None:
        with self.assertRaises(TaskPacketValidationError):
            TaskPacket.from_dict(
                {
                    'objective': '',
                    'scope': '',
                    'repo': '',
                    'branch_policy': '',
                    'acceptance_tests': ['ok', ''],
                    'commit_policy': '',
                    'reporting_contract': '',
                    'escalation_policy': '',
                }
            )

    def test_worker_create_run_resume_status_and_close(self) -> None:
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            storage = root / '.workers'
            packet_path = root / 'task.json'
            packet = {
                'objective': 'Create a note',
                'scope': 'workspace root',
                'repo': 'temp-repo',
                'branch_policy': 'stay on current branch',
                'acceptance_tests': ['python3 -c "print(\'verify ok\')"'],
                'commit_policy': 'no commit',
                'reporting_contract': 'report changed files',
                'escalation_policy': 'stop on errors',
            }
            packet_path.write_text(json.dumps(packet), encoding='utf-8')

            worker = create_worker(root=root, directory=storage)
            self.assertEqual(worker.state, 'ready')

            fake_result = AgentRunResult(
                content='Created notes.txt',
                turns=2,
                tool_calls=('edit_file',),
                tool_trace=(
                    {
                        'turn': 1,
                        'name': 'edit_file',
                        'arguments': {'path': 'notes.txt', 'new_text': 'hello'},
                        'success': True,
                        'output': 'Updated notes.txt',
                        'metadata': {'path': 'notes.txt', 'created': True},
                        'error': None,
                    },
                ),
                stop_reason='completed',
                trace_events=('tool_call[1]=edit_file {"path":"notes.txt"}',),
            )
            with patch('src.worker_api.run_agent_task', return_value=fake_result):
                ran = run_worker(worker.worker_id, packet_path, directory=storage, trace=True)

            self.assertEqual(ran.state, 'finished')
            self.assertEqual(ran.last_result['stop_reason'], 'completed')
            self.assertIn('notes.txt', ran.last_result['changed_files'])
            self.assertTrue(ran.last_result['verification']['acceptance_tests'][0]['success'])

            status = load_worker(worker.worker_id, directory=storage)
            self.assertEqual(status.worker_id, worker.worker_id)
            self.assertIsNotNone(status.last_packet)

            with patch('src.worker_api.run_agent_task', return_value=fake_result):
                resumed = resume_worker(worker.worker_id, directory=storage, trace=False)
            self.assertEqual(resumed.run_count, 2)

            workers = list_workers(directory=storage)
            self.assertEqual(len(workers), 1)
            self.assertEqual(workers[0].worker_id, worker.worker_id)

            closed = close_worker(worker.worker_id, directory=storage)
            self.assertEqual(closed.state, 'closed')

    def test_worker_cli_create_and_status(self) -> None:
        from src.main import main

        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            with patch('src.main.create_worker') as create_worker_mock:
                create_worker_mock.return_value = create_worker(root=root, directory=root / '.workers')
                with patch('sys.stdout', new=io.StringIO()):
                    exit_code = main(['worker', 'create', '--root', str(root)])
            self.assertEqual(exit_code, 0)

    def test_worker_cli_list(self) -> None:
        from src.main import main

        with patch('src.main.list_workers', return_value=[]):
            with patch('sys.stdout', new=io.StringIO()):
                exit_code = main(['worker', 'list'])
        self.assertEqual(exit_code, 0)

    def test_agent_cli_invokes_agent_loop(self) -> None:
        from src.main import main

        with patch('src.main.run_agent_task') as run_agent_task_mock:
            run_agent_task_mock.return_value.content = 'agent output'
            run_agent_task_mock.return_value.trace_events = ('tool_call[1]=read_file {"path":"src/main.py"}',)
            with patch('sys.stdout', new=io.StringIO()):
                with patch('sys.stderr', new=io.StringIO()):
                    exit_code = main(['agent', 'inspect repo', '--trace'])
        self.assertEqual(exit_code, 0)
        run_agent_task_mock.assert_called_once()


if __name__ == '__main__':
    unittest.main()
