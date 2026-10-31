#!/usr/bin/env python3
"""Check curated Gospel movements against a generated runtime database.

Run after db gen-names; pass the frontend asset database to check the bundle too.
Uses only Python's standard library and opens the database read-only.
"""
import argparse
import json
import sqlite3
from pathlib import Path


def check(data, database):
    db = sqlite3.connect(database.resolve().as_uri() + '?mode=ro', uri=True)
    places = json.loads((data / 'place_overrides.json').read_text())
    journeys = json.loads((data / 'journeys.json').read_text())['journeys']
    overrides = json.loads((data / 'event_overrides.json').read_text())
    sequences = json.loads((data / 'event_sequences.json').read_text())['sequences']
    for record in places['added']:
        found = db.execute('SELECT name, origin FROM name_entity WHERE entity_id=?',
                           (record['id'],)).fetchone()
        assert found == (record['name'], 'Haqor estimate'), (record, found)
        locs = db.execute('SELECT latitude, longitude, estimated FROM name_location WHERE entity_id=?',
                          (record['id'],)).fetchall()
        assert locs and all(-90 <= lat <= 90 and -180 <= lon <= 180 and estimated == 1
                            for lat, lon, estimated in locs), record
        assert not db.execute('SELECT 1 FROM word_name WHERE entity_id=?',
                              (record['id'],)).fetchone(), record
    for record in places['places']:
        if record['name'] not in ('Ephraim', 'Gerasa', 'Emmaus'):
            continue
        actual = db.execute('SELECT l.latitude,l.longitude,l.label,l.estimated FROM name_location l '
                            'JOIN name_entity e USING(entity_id) WHERE e.name=? ORDER BY l.ord',
                            (record['name'],)).fetchall()
        expected = [(l['latitude'],l['longitude'],l['label'],int(l.get('estimated',False)))
                    for l in record['locations']]
        assert actual == expected, (record['name'], actual, expected)
    # Compare the actual ordered stops, not only journey counts.
    for journey in journeys:
        if not journey['name'].startswith('Jesus'):
            continue
        stops = db.execute('SELECT s.ref,s.label,s.by_sea,s.note FROM journey_stop s '
                           'JOIN journey j USING(journey_id) WHERE j.name=? ORDER BY s.ord',
                           (journey['name'],)).fetchall()
        assert len(stops) == len(journey['stops']), journey['name']
        for source, (ref, label, sea, note) in zip(journey['stops'], stops):
            book, chapter, verse = source['ref'].split('.')
            packed = ({'Mat':40,'Mrk':41,'Luk':42,'Jhn':43,'Act':44}[book] << 16) | (int(chapter) << 8) | int(verse)
            assert (ref,label,sea,note) == (packed,source.get('label',''),int(source.get('by')=='sea'),source.get('note','')), source
            assert db.execute('SELECT 1 FROM verse WHERE ref=?',(ref,)).fetchone(), source
    for event in overrides['added']:
        if 10101 <= event['id'] <= 10125:
            assert db.execute('SELECT title FROM bible_event WHERE event_id=?',
                              (event['id'],)).fetchone() == (event['title'],), event
    ministry = next(s for s in sequences if s['id']=='ministry')
    for title, detail in ministry['events'].items():
        if 'Jesus' in detail.get('tracks',[]):
            assert detail.get('places'), title
    assert 'tracks' not in ministry['events']['Chief Priests Conspire Against Jesus']
    assert ministry['events']['Blind Man Healed']['places'] == ['Jerusalem@Gen.14.18']
    def precedes(before, after):
        return db.execute('WITH RECURSIVE later(id) AS ('
                          'SELECT after_id FROM event_sequence_edge WHERE before_id=? '
                          'UNION SELECT e.after_id FROM event_sequence_edge e JOIN later l ON e.before_id=l.id'
                          ') SELECT EXISTS(SELECT 1 FROM later WHERE id=?)',(before,after)).fetchone()[0]
    assert not precedes(10115,10116) and not precedes(10116,10115), 'Galilean appearances must remain unordered'
    assert precedes(10120,10122) and precedes(10123,10124) and precedes(10124,10125)
    assert not db.execute('SELECT 1 FROM event_sequence_edge WHERE before_id=10104 OR after_id=10104').fetchone(), 'Chorazin has no narrated itinerary'
    assert not db.execute('SELECT 1 FROM event_sequence_track WHERE event_id=434 AND track="Jesus"').fetchone()
    # Each trial movement must survive as a distinct place, with Pilate repeated.
    sites = [db.execute('SELECT entity_id FROM event_sequence_place WHERE event_id=? ORDER BY ord',(id,)).fetchone()[0]
             for id in range(10120,10126)]
    assert len(set(sites)) == 5 and sites[3] == sites[5], sites
    db.close()
    print(f'Gospel coverage verified: {database}; {len(places["added"])} estimated sites, 20 added journeys, 19 added events.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, default=Path(__file__).resolve().parents[1] / 'data')
    parser.add_argument('--db', type=Path)
    args = parser.parse_args()
    check(args.data, args.db or args.data / 'haqor.db')
