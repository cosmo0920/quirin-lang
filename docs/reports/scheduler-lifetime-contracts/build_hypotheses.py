#!/usr/bin/env python3
"""Rebuild the report-owned proposed symbol inventory, never provider contracts."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parent
ROLES={}
for text in (ROOT/'scheduler.qgd').read_text().split(';')[1:]:
    text=text.strip()
    if not text: continue
    if '||->' in text:
        op,cases=text.split('||->');ROLES[op.strip()]='operation'
        for case in cases.split('/'):ROLES[case.strip()]='case'
    elif '|->' in text:
        left,op=text.split('|->');terms=[s.strip() for s in left.strip()[1:-1].split('∧')]
        ROLES[terms[0]]='environment'
        for term in terms[1:]:ROLES[term]='prerequisite'
        ROLES[op.strip()]='operation'
    elif '!->' in text:
        condition,op=text.split('!->');ROLES[condition.strip()]='condition';ROLES[op.strip()]='operation'
    else:
        case,post=text.split('->|');ROLES[case.strip()]='case';ROLES[post.strip()]='postcondition'
PROPS={
'environment':'Exact seven pinned source files, correct ABI/layout/libco/TLS configuration; finite sequential model, no extraction/deployment proof.',
'worker_tls':'Current OS thread owns selected scheduler; worker TLS identifies it. Current-thread TLS is absent or a live modeled parent token, never a callee/params/borrowed allocation token. Cleanup itself ignores sched.',
'valid_membership':'Valid circular sentinels, reciprocal links, distinct live embedded nodes, exact disjoint membership and valid saved successor.',
'suspended_coroutines':'Every retiring coroutine is suspended, not co_active; no concurrent resume, destruction or re-entry.',
'exact_parent_embedding':'Node is _head of the HTTP object embedded in exactly one live sufficiently large parent allocation; parent field identifies it.',
'borrowed_chain_live':'Session, channel and event survive through reset; destruction releases none of these borrowed objects.',
'parent_live':'Parent allocation has exclusive release authority and is neither borrowed nor already freed.',
'callee_live':'Libco callee is initialized, live, compatible with co_delete and not running.',
'request_live':'Request/session and writable fields remain live on every path reaching their uses.',
'allocation_extent':'Allocation arithmetic, alignment and MK_THREAD_DATA extent agree; failure/data-null alternatives retained.',
'callback_footprint':'Every optional cb_destroy and transitive effect has independently established returning trace preserving parent, callee, borrowed chain, complete list/saved-successor footprint until enclosing destruction; no neutral/circular assumption.',
'callback_dispatch_closure':'Library callback code/data/request/session lifetime and all transitive effects separately established; plugin dispatch remains FIXME, allocation/event failures retained.',
'no_later_alias_use':'All external aliases, request->thread and channel.thread are retired or never read/resumed after parent free. Source does not clear them.',
'no_concurrent_writer':'No concurrent list, coroutine, borrowed-chain or TLS mutation.',
'tls_params_owned':'Params TLS is NULL or independently allocated live releasable storage, disjoint from owned/borrowed/callback resources.',
'tls_api_success':'TLS initialization/get/set configuration established; ignored pthread key/set return values must succeed, not inferred from source binding.',
'invalid_membership':'Positive absent/duplicate/wrong-list/stale membership forbids admission; source has no complete membership guard.',
'retiring_current_tls':'Current mk_thread TLS aliases retiring parent; bounded consumer refuses because source never clears it. Broader rebind protocol requires proof.',
'cleanup':'Proposed admission of bounded safe-list iteration, counting once per completed destroy.',
'destroy':'Proposed admission of params free/clear, unlink, event reset, optional callback, callee delete and parent free in source order.',
'create':'Proposed admission of source creation including every early-return and unchecked coroutine outcome; finite cleanup checker does not validate creation.',
'purge':'Proposed admission of close assignment and active-to-purge migration; no ownership release.',
'empty_list':'No selected member and no destroy call; nonnull params TLS remains untouched.',
'nonempty_list':'One or more admitted suspended members destroyed once in original safe-iteration order.',
'original_state_unchanged':'Empty cleanup has no modeled effects, return count zero, no params free.',
'selected_list_empty_and_borrows_live':'Selected parents/callees retired once; other list/borrowed chain survive; events reset; count matches original length; params freed at most once.',
'parent_callee_retired_borrows_live':'Unlink, event reset, callee delete, parent free; borrowed objects survive; no params existed.',
'parent_callee_params_retired_borrows_live':'Free/clear params, unlink, event reset, callee delete, parent free; borrowed objects survive.',
'callback_checked_parent_callee_retired':'No params; explicit callback after reset and before callee/parent retirement; independent footprint evidence required.',
'callback_checked_parent_callee_params_retired':'Free/clear params, unlink/reset, explicit callback, callee delete, parent free; borrowed objects survive.',
'missing_scheduler':'Create scheduler lookup NULL before parent allocation.',
'allocation_failed':'mk_thread_new returns NULL, producing no new owner.',
'data_null':'Explicit MK_THREAD_DATA NULL test returns NULL after parent allocation without source free; reachability under defined layout remains Open.',
'coroutine_creation_failed':'co_create NULL after insertion/request assignment; source proceeds to params/switch unchecked. No successful creation or rollback inferred.',
'created':'Valid parent/embedded data/callee, params initialized, initialization context switch returns; all dependent effects established independently.',
'creation_unchanged':'Missing scheduler creates no owner and edits no list/request.',
'creation_no_new_owner':'Allocation failure creates no cleanup duty; pre-existing caller resources survive.',
'allocated_parent_unresolved':'Allocated parent remains outstanding on explicit data-null edge unless independently proved infeasible.',
'linked_owner_with_unresolved_switch':'Linked parent and request alias exist before unchecked callee failure; attempted switch effects and termination Open.',
'linked_parent_and_borrowed_request':'Parent owns embedded HTTP object/callee; request/session/handler data borrowed; active list gains one node, conditional on creation prerequisites.',
'missing_purge_scheduler':'Purge scheduler lookup NULL returns -1 before close/list edit.',
'moved_to_purge':'Same-owner scheduler found, close assigned, node unlinked then inserted in purge list; zero return.',
'original_lists_unchanged':'Missing-scheduler purge leaves lists, close and ownership unchanged.',
'active_unlinked_purge_member':'Exactly one active node migrates to purge; parent ownership unchanged; no free.'}
for case in ('tls_absent_callback_absent','tls_present_callback_absent','tls_absent_callback_present','tls_present_callback_present'):
    PROPS[case]='Explicit destroy branch: '+case.replace('_',' ')+'. Presence/absence positive; callback effects independently required.'
assert set(PROPS)==set(ROLES),(set(ROLES)-set(PROPS),set(PROPS)-set(ROLES))
(ROOT/'hypotheses.json').write_text(json.dumps({'version':1,'authority':'proposed source-bound decomposition only; no original graph or production declaration modified',
'bounds':{'threads':32,'live_resources':512,'callback_effects_per_thread':64,'trace_steps':2560},
'symbols':{key:{'role':ROLES[key],'proposition':PROPS[key],'source_truth':'Open'} for key in sorted(ROLES)},
'lifecycle_metadata':{'creation':'|^+ introduces resource/hypothesis metadata; no allocation fact follows', 'retirement':'|^- retires resource/hypothesis metadata; no C free/global negation follows'}},indent=2)+'\n')
