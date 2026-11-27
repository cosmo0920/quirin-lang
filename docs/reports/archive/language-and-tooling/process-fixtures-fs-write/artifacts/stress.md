| Set | Binary sha256 | Amplifier | Runs | Deviating | With ETXTBSY | Child stalls |
|---|---|---|---:|---:|---:|---:|
| base-cycle20_verifier_plugins | `12b45fc7d8d3…` | close 20 ms, child stall 20 ms | 20 | 0 | 0 | 93 |
| base-final-cycle20_verifier_plugins | `12b45fc7d8d3…` | close 100 ms, child stall 50 ms | 50 | 1 | 1 | 283 |
| base-final-standalone_host | `417583774df3…` | close 100 ms, child stall 50 ms | 50 | 0 | 0 | 269 |
| base-final-subcommand_plugins | `0463c3a168ae…` | close 20 ms, child stall 20 ms | 50 | 15 | 9 | 208 |
| base-standalone_host | `417583774df3…` | close 20 ms, child stall 20 ms | 20 | 0 | 0 | 109 |
| base-subcommand_plugins | `0463c3a168ae…` | close 20 ms, child stall 20 ms | 20 | 7 | 7 | 71 |
| base-wide-cycle20_verifier_plugins | `12b45fc7d8d3…` | close 100 ms, child stall 50 ms | 20 | 2 | 2 | 107 |
| tip-final-cycle20_verifier_plugins | `cd39c673b9ba…` | close 100 ms, child stall 50 ms | 50 | 0 | 0 | 545 |
| tip-final-standalone_host | `5425693655cf…` | close 100 ms, child stall 50 ms | 50 | 0 | 0 | 578 |
| tip-final-subcommand_plugins | `f62c9a47ea6d…` | close 20 ms, child stall 20 ms | 50 | 0 | 0 | 305 |

base-final-cycle20_verifier_plugins: failing tests {'vp01_vp02_process_proposals_reach_the_firewall_and_cannot_mint_safety': 1}

base-final-subcommand_plugins: failing tests {'uncertified_claims_are_pending': 11, 'no_claim_lowers_an_existing_unsafe': 11, 'an_external_plugin_runs_and_its_artifacts_are_recorded': 6, 'oversize_output_is_unknown': 8, 'an_artifact_outside_the_output_is_unknown': 4, 'a_failed_plugin_joined_with_safe_is_unknown': 2, 'a_nonzero_exit_is_unknown': 2, 'the_environment_does_not_leak': 8, 'malformed_json_is_unknown': 5, 'a_timeout_is_unknown_and_kills_the_process_group': 2, 'a_version_mismatch_is_unknown_with_a_named_reason': 2}
  deviating logs without ETXTBSY: ['0-0-0-0-0-0-20-020.log', '0-0-0-0-0-0-20-021.log', '0-0-0-0-0-0-20-024.log', '0-0-0-0-0-0-20-026.log', '0-0-0-0-0-0-20-040.log', '0-0-0-0-0-0-20-047.log']

base-subcommand_plugins: failing tests {'no_claim_lowers_an_existing_unsafe': 7, 'a_version_mismatch_is_unknown_with_a_named_reason': 3, 'oversize_output_is_unknown': 5, 'uncertified_claims_are_pending': 7, 'an_external_plugin_runs_and_its_artifacts_are_recorded': 5, 'the_environment_does_not_leak': 6, 'an_artifact_outside_the_output_is_unknown': 3, 'malformed_json_is_unknown': 5, 'a_nonzero_exit_is_unknown': 5, 'a_failed_plugin_joined_with_safe_is_unknown': 1, 'a_timeout_is_unknown_and_kills_the_process_group': 1}

base-wide-cycle20_verifier_plugins: failing tests {'vp01_vp02_process_proposals_reach_the_firewall_and_cannot_mint_safety': 2}
