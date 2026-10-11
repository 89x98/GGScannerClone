"""Local-only mission audit. Never sends capture data over the network.

Usage: python mission_audit.py DATAMINE_REPO DECRYPTED_SESSION [--output FILE]
Replay raw frames with examples/analyze_dump first. Output contains aggregate
counts and static configuration examples only, never player IDs or packet bytes.
Wire tags reviewed against IceDynamix/reliquary revision
0b8c1d4f8eb5ae9722b5c3273e36a464b7188a38 (HSR 4.6).
This is research tooling, not a production mission decoder.
"""
import argparse
import collections
import json
import pathlib
import re
import subprocess


def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args])


def read_blobs(repo, paths):
    data = subprocess.run(
        ['git', '-C', str(repo), 'cat-file', '--batch'],
        input=''.join('HEAD:' + p + '\n' for p in paths).encode(),
        stdout=subprocess.PIPE, check=True).stdout
    pos = 0
    for path in paths:
        end = data.index(b'\n', pos)
        size = int(data[pos:end].split()[-1])
        pos = end + 1
        yield path, json.loads(data[pos:pos + size])
        pos += size + 1


def varint(data, pos):
    value = 0
    for shift in range(0, 70, 7):
        b = data[pos]
        pos += 1
        value |= (b & 127) << shift
        if not b & 128:
            return value, pos
    raise ValueError('invalid varint')


def fields(data):
    out = []
    pos = 0
    try:
        while pos < len(data):
            tag, pos = varint(data, pos)
            number, wire = tag >> 3, tag & 7
            if not number:
                return None
            if wire == 0:
                value, pos = varint(data, pos)
            elif wire == 2:
                size, pos = varint(data, pos)
                value = data[pos:pos + size]
                pos += size
            elif wire in (1, 5):
                size = 8 if wire == 1 else 4
                value = data[pos:pos + size]
                pos += size
            else:
                return None
            if pos > len(data):
                return None
            out.append((number, wire, value))
        return out
    except (IndexError, ValueError):
        return None


def packed(fs, number):
    out = []
    for n, w, v in fs:
        if n != number:
            continue
        if w == 0:
            out.append(v)
        elif w == 2:
            pos = 0
            while pos < len(v):
                x, pos = varint(v, pos)
                out.append(x)
        else:
            raise ValueError('not a uint32 list')
    return out


def records(fs, tag, id_tag, allowed, catalog):
    out = []
    for n, w, v in fs:
        if n != tag:
            continue
        child = fields(v) if w == 2 else None
        if not child or any(n not in allowed for n, _, _ in child):
            return None
        values = {n: v for n, w, v in child if w == 0}
        if not isinstance(values.get(id_tag), int) or values[id_tag] <= 0:
            return None
        out.append(values)
    return out


def audit(repo, session, graph_path=None):
    paths = git(repo, 'ls-tree', '-r', '--name-only', 'HEAD', 'Config/Level/Mission').decode().splitlines()
    selected = [p for p in paths if re.search(r'/MissionInfo_\d+(?:\.partial)?\.json$', p)]
    tables = dict(read_blobs(repo, ['ExcelOutput/' + n + '.json' for n in
                                  ['MainMission', 'SubMission', 'QuestData', 'AchievementData',
                                   'DailyQuest', 'BattlePassQuest', 'CycleQuest', 'FinishWay', 'LinearQuest']]))
    main = {r['MainMissionID']: r for r in tables['ExcelOutput/MainMission.json']}
    sub = {r['SubMissionID']: r for r in tables['ExcelOutput/SubMission.json']}
    quest = {r['QuestID']: r for r in tables['ExcelOutput/QuestData.json']}
    ownership = collections.defaultdict(set)
    configs = {}
    config_sources = {}
    conflicting_variants = []
    info = {}
    for path, obj in read_blobs(repo, selected):
        if 'MainMissionID' in obj:
            info[obj['MainMissionID']] = obj
        for row in obj.get('SubMissionList', []):
            ownership[row['ID']].add(row['MainMissionID'])
            if row['ID'] in configs and configs[row['ID']] != row:
                conflicting_variants.append({'id': row['ID'], 'sources': [config_sources[row['ID']], path]})
            configs[row['ID']] = row
            config_sources[row['ID']] = path
    take = collections.Counter(r.get('TakeType', '') for r in configs.values())
    covered = set().union(*ownership.values())
    branch = [(sid, row) for sid, row in configs.items()
              if len(row.get('TakeParamIntList', [])) > 1 and row.get('TakeType') in ('AnySequence', 'AllSequence')]
    report = {
        'datamine_revision': git(repo, 'rev-parse', 'HEAD').decode().strip(),
        'static': {'main_rows': len(main), 'sub_rows': len(sub), 'mission_info_files': len(selected),
                   'main_with_info': len(set(info) & set(main)), 'main_with_owned_sub': len(covered & set(main)),
                   'sub_with_explicit_owner': len(set(ownership) & set(sub)),
                   'configured_sub': len(configs), 'multiple_owners': sum(len(v) > 1 for v in ownership.values()),
                   'conflicting_sub_config_variants': conflicting_variants,
                   'main_types': dict(collections.Counter(r.get('Type') for r in main.values())),
                   'take_types': dict(take), 'multi_sequence_conditions': len(branch),
                   'example_1000101': info.get(1000101),
                   'example_100020103_owner': sorted(ownership.get(100020103, [])),
                   'branch_examples': [row for _, row in branch[:3]]},
    }
    if graph_path:
        # Preserve typed conditions verbatim. Do not flatten alternatives into
        # an unconditional list or treat every integer parameter as a Mission ID.
        graph = {'revision': report['datamine_revision'], 'main': main,
                 'main_info': info, 'sub': {sid: {'source': config_sources[sid], 'config': row}
                                           for sid, row in configs.items()},
                 'ownership': {sid: sorted(owners) for sid, owners in ownership.items()}}
        graph_path.write_text(json.dumps(graph, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    observations = []
    mission_hits = []
    status_hits = []
    structural_candidates = []
    finished_seen = set()
    main_doing = set()
    sub_seen = {}
    main_status_seen = {}
    bodies = sorted(session.glob('hsr_*.bin'))
    for path in bodies:
        fs = fields(path.read_bytes())
        if not fs:
            continue
        tags = {n for n, _, _ in fs}
        # Exact reviewed GetQuestDataScRsp shape, corroborated by catalog IDs.
        if tags <= {8, 10}:
            qs = records(fs, 10, 12, {1, 3, 10, 12, 14}, quest)
            if qs and len(qs) > 100 and all(r[12] in quest for r in qs):
                observations = qs
        # Reviewed GetMissionDataScRsp: field 8 Mission, 9 completed main,
        # 11 MainMission. No guessed semantics for obfuscated fields 2/13/15.
        if tags <= {2, 4, 6, 8, 9, 11, 13, 15} and (8 in tags or 9 in tags or 11 in tags):
            try:
                ms = records(fs, 8, 8, {1, 4, 8}, sub)
                mm = records(fs, 11, 6, {1, 6, 14}, main)
                finished = packed(fs, 9)
                if ms is not None and mm is not None and all(x in main for x in finished):
                    if ms and mm and len(finished) > 10:
                        finished_seen.update(finished)
                        main_doing.update(r[6] for r in mm if r.get(14, 0) == 1)
                        sub_seen.update((r[8], r.get(1, 0)) for r in ms)
                        mission_hits.append({'file': path.name, 'sub_records': len(ms),
                                             'sub_ids_in_text_table': sum(r[8] in sub for r in ms),
                                             'main_records': len(mm), 'finished_main': len(finished),
                                             'sub_statuses': dict(collections.Counter(r.get(1, 0) for r in ms)),
                                             'main_statuses': dict(collections.Counter(r.get(14, 0) for r in mm)),
                                             'unnamed_field_15_uint32_count': len(packed(fs, 15)),
                                             'unnamed_field_15_static_sub_matches': sum(x in ownership for x in packed(fs, 15)),
                                             'finished_main_types': dict(collections.Counter(main[x].get('Type') for x in finished))})
            except (ValueError, IndexError):
                pass
        if tags <= {2, 3, 5, 7, 9, 12, 14} and (2 in tags or 14 in tags):
            try:
                ms = records(fs, 2, 8, {1, 4, 8}, sub)
                lists = {str(n): packed(fs, n) for n in (3, 5, 9, 14)}
                if ms is not None and all(x in main for v in lists.values() for x in v):
                    if ms:
                        sub_seen.update((r[8], r.get(1, 0)) for r in ms)
                        for n, status in ((3, 'unfinished'), (5, 'curversion_finished'),
                                          (9, 'disabled'), (14, 'finished')):
                            main_status_seen.update((x, status) for x in lists[str(n)])
                        status_hits.append({'file': path.name, 'sub_records': len(ms),
                                            'sub_statuses': dict(collections.Counter(r.get(1, 0) for r in ms)),
                                            'main_lists': {n: len(v) for n, v in lists.items()}})
            except (ValueError, IndexError):
                pass
        # Diagnostic only: numeric overlap is not a mission schema decoder.
        overlaps = {}
        for n, w, v in fs:
            if w == 2:
                child = fields(v)
                if child:
                    hits = [x for _, wire, x in child if wire == 0 and x in sub]
                    if hits:
                        overlaps[n] = overlaps.get(n, 0) + len(hits)
        if overlaps:
            structural_candidates.append({'file': path.name, 'sub_id_scalar_hits': overlaps})
    achievement = {r['QuestID'] for r in tables['ExcelOutput/AchievementData.json']}
    ways = {r['ID']: r for r in tables['ExcelOutput/FinishWay.json']}
    finish = {qid: ways[row['FinishWayID']] for qid, row in quest.items()
              if ways.get(row.get('FinishWayID'), {}).get('FinishType') == 'FinishMission' and qid not in achievement}
    observed = {r[12]: r for r in observations}
    daily = {q for r in tables['ExcelOutput/DailyQuest.json'] for q in r.get('QuestList', [])}
    battlepass = {r['ID'] for r in tables['ExcelOutput/BattlePassQuest.json']}
    cycle = {q for r in tables['ExcelOutput/CycleQuest.json'] if not r.get('IsNonPeriodic', False)
             for q in r.get('QuestList', [])}
    periodic = daily | battlepass | cycle
    report['packets'] = {'decrypted_bodies': len(bodies), 'quest_records': len(observations),
                         'mission_data_matches': mission_hits, 'mission_status_matches': status_hits,
                         'latest_sub_status_counts': dict(collections.Counter(sub_seen.values())),
                         'distinct_sub_status_ids': len(sub_seen),
                         'sub_ids_with_static_owner': len(set(sub_seen) & set(ownership)),
                         'queried_main_status_counts': dict(collections.Counter(main_status_seen.values())),
                         'doing_main_types': dict(collections.Counter(main[x].get('Type') for x in main_doing)),
                         'numeric_overlap_candidates': structural_candidates}
    report['quest'] = {'id_overlap_main': sum(r[12] in main for r in observations),
                       'id_overlap_sub': sum(r[12] in sub for r in observations),
                       'quest_condition_keys': sorted({k for r in quest.values() for k in r if 'Type' in k or 'Param' in k}),
                       'finish_mission_condition_rows': len(finish),
                       'observed_finish_mission_rows': len(set(finish) & set(observed)),
                       'finish_mission_observed_statuses': dict(collections.Counter(observed[q].get(1, 0) for q in finish if q in observed)),
                       'periodic_catalog': len(periodic),
                       'observed_periodic_nonachievement': len((set(observed) - achievement) & periodic),
                       'observed_nonperiodic_nonachievement': len(set(observed) - achievement - periodic),
                       'linear_multi_id_rows': sum(len(r.get('QuestList', [])) > 1 for r in tables['ExcelOutput/LinearQuest.json']),
                       'finish_mission_examples': list(finish.values())[:3],
                       'id_overlap_examples': [{'quest_id': r[12], 'quest_condition': ways.get(quest[r[12]].get('FinishWayID')),
                                               'main_mission_type': main[r[12]].get('Type')}
                                              for r in observations if r[12] in main][:3],
                       'completed_single_finish_mission_conditions': sum(
                           observed[q].get(1, 0) in (2, 3) and len(row.get('ParamIntList', [])) == 1
                           and row.get('Progress') == 1 for q, row in finish.items() if q in observed),
                       'completed_single_finish_mission_already_in_direct_list': sum(
                           observed[q].get(1, 0) in (2, 3) and len(row.get('ParamIntList', [])) == 1
                           and row.get('Progress') == 1 and row['ParamIntList'][0] in finished_seen
                           for q, row in finish.items() if q in observed),
                       'daily_sample': tables['ExcelOutput/DailyQuest.json'][:1],
                       'cycle_sample': tables['ExcelOutput/CycleQuest.json'][:1]}
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('datamine_repo', type=pathlib.Path)
    parser.add_argument('decrypted_session', type=pathlib.Path)
    parser.add_argument('--output', type=pathlib.Path)
    parser.add_argument('--graph', type=pathlib.Path, help='save lossless static conditions and explicit ownership')
    args = parser.parse_args()
    result = json.dumps(audit(args.datamine_repo, args.decrypted_session, args.graph), ensure_ascii=False, indent=2)
    if args.output:
        args.output.write_text(result + '\n', encoding='utf-8')
    else:
        print(result)
