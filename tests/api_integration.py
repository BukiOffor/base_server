"""Run against a disposable server/database and the local auth-delivery simulation.
TGA_TEST_URL=http://127.0.0.1:9000/api/v1 python3 tests/api_integration.py
Never point this test at production: it creates and deletes fixture records.
"""
import concurrent.futures
import datetime
import io
import json
import os
import unittest
import urllib.error
import urllib.request
import uuid
import zipfile
import xml.etree.ElementTree as ET

BASE = os.environ.get('TGA_TEST_URL', 'http://127.0.0.1:9000/api/v1')
DELIVERY = os.environ.get('TGA_TEST_DELIVERY', 'http://127.0.0.1:55560/auth-delivery/messages')

class APIIntegration(unittest.TestCase):
    def request(self, method, path, body=None, status=200, token=True, raw=False, headers=None):
        h = {'Content-Type': 'application/json'}
        if token and getattr(self, 'token', None):
            h['Authorization'] = 'Bearer ' + (self.token if token is True else token)
        h.update(headers or {})
        data = body if isinstance(body, bytes) else json.dumps(body).encode() if body is not None else None
        req = urllib.request.Request(BASE + path, data=data, method=method, headers=h)
        try:
            response = urllib.request.urlopen(req, timeout=30)
        except urllib.error.HTTPError as e:
            response = e
        payload = response.read()
        self.assertEqual(response.status, status, f'{method} {path}: {payload[:500]!r}')
        self.assertNotIn('Set-Cookie', response.headers)
        if raw:
            return payload, response.headers
        return json.loads(payload) if payload else None

    def messages(self):
        return json.load(urllib.request.urlopen(DELIVERY, timeout=10))

    def test_roster_contract(self):
        login = self.request('POST', '/auth/login', {'email': 'admin@example.test', 'password': 'local-test-password-123'}, token=False)
        self.token = login['accessToken']
        self.request('GET', '/auth/me')
        self.request('GET', '/members', status=401, token=False)
        self.request('GET', '/members', status=401, token=False, headers={'Cookie': 'access_token=' + self.token})
        self.request('GET', '/members', status=401, token='bad-token')
        self.request('POST', '/admin/reset-demo', status=404)
        refreshed = self.request('POST', '/auth/refresh', {'refreshToken': login['refreshToken']}, token=False)
        self.assertNotEqual(login['refreshToken'], refreshed['refreshToken'])
        self.request('POST', '/auth/refresh', {'refreshToken': login['refreshToken']}, token=False, status=401)
        self.request('GET', '/members', token=refreshed['refreshToken'], status=401)
        self.token = refreshed['accessToken']
        self.request('POST', '/auth/refresh', {'refreshToken': self.token}, token=False, status=401)
        for invalid in [{'firstName': 5}, {'firstName': 'Bad', 'lastName': 'Input', 'phone': 'not-a-number'}, {'firstName': 'Bad', 'lastName': 'Input', 'phone': '08031234567', 'role': 'leader'}]:
            self.request('POST', '/members', invalid, status=400)
        self.request('GET', '/members?page=0', status=400)
        self.request('GET', '/members?dir=wrong', status=400)
        suffix = uuid.uuid4().hex[:8]
        date = (datetime.date.today() + datetime.timedelta(days=60)).isoformat()
        def event(name):
            return self.request('POST', '/events', {'name': name, 'code': suffix, 'series': suffix, 'date': date, 'venue': 'Test Hall', 'reportingTime': '07:30', 'startTime': '08:45', 'session': 'morning', 'day': 1, 'useTemplate': True}, status=201)
        ev = event('Integration ' + suffix)
        eid = ev['id']
        self.request('PUT', '/settings/current-event', {'eventId': eid})
        self.request('DELETE', '/events/' + eid, status=422)
        zones = self.request('GET', f'/events/{eid}/zones?include=positions,stats')
        self.assertEqual(len(zones), 121)
        self.assertEqual(sum(bool(p.get('spot')) for z in zones for p in z['positions']), 312)
        section = next(z for z in zones if z['mapRef'] == 'S41')
        pos, pos2 = section['positions'][:2]
        aisle = next(z for z in zones if z['mapRef'] == 'A1')
        self.request('POST', f'/events/{eid}/zones', {'name': 'Other', 'kind': 'section', 'mapRef': 'S41'}, status=409)
        self.request('PATCH', '/positions/' + pos['id'], {'required': 2}, status=422)
        self.request('PATCH', '/positions/' + pos['id'], {'required': None}, status=400)
        self.request('POST', '/zones/' + aisle['id'] + '/positions', {'name': 'Invalid Seat', 'spot': 'FL'}, status=422)
        self.request('POST', f'/events/{eid}/zones/apply-template', {'mode': 'add-missing'})
        self.assertEqual(len(self.request('GET', f'/events/{eid}/zones')), 121)
        self.request('GET', f'/public/events/{eid}/map', token=False, status=404)
        phones = ['0803' + str(int(suffix, 16) % 1000000).zfill(6) + str(i) for i in range(6)]
        members = []
        for i in range(4):
            m = self.request('POST', '/members', {'firstName': 'Ṣọlá Adéyẹmí Longname' if i == 2 else 'Fixture' + str(i), 'lastName': suffix, 'phone': phones[i], 'email': f'fixture{i}@example.test', 'memberId': suffix + str(i)}, status=201)
            members.append(m)
        page = self.request('GET', '/members?q=' + suffix + '&pageSize=2&page=2&sort=name&dir=asc')
        self.assertEqual(page['total'], 4); self.assertEqual(len(page['items']), 2)
        m1, m2, m3, co = members
        self.request('POST', '/members', {'firstName': 'Duplicate', 'lastName': 'Test', 'phone': '+234' + phones[0][1:]}, status=409)
        self.assertFalse(self.request('GET', '/members/check-phone?phone=' + phones[0])['available'])
        def assign(m, p=pos, **extra):
            return {'memberId': m['id'], 'zoneId': section['id'], 'positionId': p['id'], **extra}
        a = self.request('POST', f'/events/{eid}/assignments', assign(m1), status=201)['assignment']
        hidden = self.request('POST', '/public/lookup', {'phone': phones[0]}, token=False)
        self.assertNotIn('view', hidden['sessions'][0])
        self.request('POST', f'/events/{eid}/publish')
        self.assertEqual(self.request('GET', '/public/current-event', token=False)['stats']['assigned'], 1)
        self.request('POST', f'/events/{eid}/assignments', assign(m2, replace=False), status=409)
        self.request('POST', '/public/assignments/' + a['id'] + '/confirm', {'phone': phones[1]}, token=False, status=403)
        self.request('PATCH', '/settings', {'showTeammatePhones': False, 'allowSelfConfirm': False})
        self.request('POST', '/public/assignments/' + a['id'] + '/confirm', {'phone': phones[0]}, token=False, status=403)
        self.request('PATCH', '/settings', {'allowSelfConfirm': True})
        self.request('POST', '/public/assignments/' + a['id'] + '/confirm', {'phone': phones[0]}, token=False)
        self.request('POST', '/public/assignments/' + a['id'] + '/unavailable', {'phone': phones[0], 'reason': 'Travel'}, token=False)
        self.request('POST', '/public/assignments/' + a['id'] + '/unavailable', {'phone': phones[0]}, token=False, status=409)
        self.assertEqual(self.request('GET', f'/events/{eid}/reports/summary')['stats']['assigned'], 0)
        public_map = self.request('GET', f'/public/events/{eid}/map', token=False)
        seat = next(s for s in public_map['spots'] if s['positionId'] == pos['id'])
        self.assertFalse(seat['taken']); self.assertTrue(seat['needsCover'])
        self.assertNotIn('occupant', seat)
        self.request('POST', f'/events/{eid}/assignments', assign(m2), status=201)
        self.request('POST', '/public/assignments/' + a['id'] + '/available', {'phone': phones[0]}, token=False, status=409)
        absences = self.request('GET', f'/events/{eid}/absences?covered=true&zoneName=Section%2041')
        self.assertEqual(len(absences), 1)
        self.assertEqual(self.request('GET', f'/events/{eid}/absences?zoneName=Section%2042'), [])
        self.assertEqual(self.request('GET', f'/events/{eid}/reports/summary')['stats']['needsCover'], 0)
        displaced = self.request('POST', f'/events/{eid}/assignments', assign(m3), status=201)
        self.assertEqual(displaced['replaced']['displacedFromSpot']['member']['id'], m2['id'])
        a3 = displaced['assignment']
        moved = self.request('PATCH', '/assignments/' + a3['id'], {'positionId': pos2['id']})['assignment']
        self.assertEqual(moved['id'], a3['id'])
        self.request('POST', '/public/assignments/' + a['id'] + '/available', {'phone': phones[0]}, token=False)
        looked = self.request('POST', '/public/lookup', {'phone': phones[0]}, token=False)
        self.assertEqual(looked['kind'], 'member')
        teammates = looked['sessions'][0]['view']['teammates']
        self.assertTrue(all('phone' not in x['member'] and 'email' not in x['member'] for x in teammates))
        for m in [m1, m3]:
            self.request('POST', f'/events/{eid}/assignments', assign(m, pos if m == m1 else pos2, isTeamLead=True), status=201)
        assignments = self.request('GET', f'/events/{eid}/assignments')['items']
        self.assertEqual(sum(a['isTeamLead'] for a in assignments), 1)
        self.request('PUT', '/members/' + co['id'] + '/role', {'role': 'coordinator', 'coordinates': ['Section 41']})
        self.request('POST', '/team/auth/request-code', {'phone': phones[3]}, token=False, status=204)
        code = next(m['value'] for m in reversed(self.messages()) if m['kind'] == 'team-code' and m['recipient'] == phones[3])
        self.request('POST', '/team/auth/verify', {'phone': phones[3], 'code': 'invalid'}, token=False, status=401)
        team_token = self.request('POST', '/team/auth/verify', {'phone': phones[3], 'code': code}, token=False)['accessToken']
        self.request('POST', '/team/auth/verify', {'phone': phones[3], 'code': code}, token=False, status=401)
        team = self.request('GET', f'/team/me?eventId={eid}', token=team_token)
        self.assertEqual(team['stats']['zones'], 1)
        self.request('GET', '/members', token=team_token, status=401)
        team_map = self.request('GET', f'/team/events/{eid}/map', token=team_token)
        self.assertEqual(len(team_map['zones']), 1)
        outside = next(s for s in team_map['spots'] if s['zoneId'] != section['id'])
        self.assertNotIn('occupant', outside)
        team_a = next(a for a in self.request('GET', f'/events/{eid}/assignments')['items'] if a['memberId'] == m3['id'])
        self.request('PATCH', '/assignments/' + team_a['id'], {'status': 'unavailable'})
        away_team = self.request('GET', f'/team/me?eventId={eid}', token=team_token)
        self.assertEqual(away_team['stats']['unavailable'], 1)
        self.assertTrue(any(n['title'] == 'Spot needs cover' for n in away_team['notifications']))
        self.request('POST', f'/events/{eid}/unpublish')
        draft_team = self.request('GET', f'/team/me?eventId={eid}', token=team_token)
        self.assertEqual(draft_team['stats']['assigned'], 0)
        self.assertTrue(all(not z['ushers'] and not z['unavailable'] for c in draft_team['coordinators'] for z in c['zones']))
        self.request('POST', f'/events/{eid}/publish')
        self.request('PATCH', '/assignments/' + team_a['id'], {'status': 'assigned'})
        self.request('POST', '/team/auth/logout', token=team_token, status=204)
        self.request('GET', '/team/me', token=team_token, status=401)
        # Unavailable rows stay out of copied rosters and exports.
        current = self.request('GET', f'/events/{eid}/assignments')['items']
        m1_assignment = next(a for a in current if a['memberId'] == m1['id'])
        self.request('PATCH', '/assignments/' + m1_assignment['id'], {'status': 'unavailable', 'unavailableReason': 'Away'})
        duplicate = self.request('POST', f'/events/{eid}/duplicate', status=201)
        copied = self.request('POST', f'/events/{duplicate["id"]}/roster/copy-from', {'fromEventId': eid})
        self.assertEqual(copied['copied'], 1)
        self.assertEqual(self.request('GET', f'/events/{duplicate["id"]}/assignments')['total'], 1)
        self.request('POST', f'/events/{eid}/roster/copy-from', {'fromEventId': duplicate['id']}, status=422)
        for fmt, magic in [('csv', b'Zone,'), ('xlsx', b'PK'), ('pdf', b'%PDF')]:
            data, h = self.request('GET', f'/events/{eid}/exports/full?format={fmt}', raw=True)
            self.assertTrue(data.startswith(magic))
            self.assertIn('attachment;', h['Content-Disposition'])
            if fmt == 'pdf' and os.environ.get('TGA_TEST_PDF'):
                with open(os.environ['TGA_TEST_PDF'], 'wb') as output:
                    output.write(data)
            if fmt == 'xlsx':
                self.assertIn('xl/worksheets/sheet1.xml', zipfile.ZipFile(io.BytesIO(data)).namelist())
        self.request('GET', f'/events/{eid}/exports/aisle', status=400)
        # CSV import preview and commit are reusable only once.
        csv_data = ('First Name,Last Name,Phone,Zone,Position\nImported,' + suffix + ',' + phones[4] + ',Section 41,Front row, right end\nDuplicate,Row,' + phones[4] + ',,\n').encode()
        boundary = 'roster-test-boundary'
        data = (f'--{boundary}\r\nContent-Disposition: form-data; name="eventId"\r\n\r\n{eid}\r\n--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="fixture.csv"\r\nContent-Type: text/csv\r\n\r\n'.encode() + csv_data + f'\r\n--{boundary}--\r\n'.encode())
        preview = self.request('POST', '/imports/preview', data, headers={'Content-Type': 'multipart/form-data; boundary=' + boundary})
        self.assertEqual(preview['summary']['error'], 1)
        counts = self.request('POST', '/imports/' + preview['importId'] + '/commit', {'updateExisting': True, 'assign': True})
        self.assertEqual(counts['added'], 1); self.assertEqual(counts['skipped'], 1)
        self.request('POST', '/imports/' + preview['importId'] + '/commit', {'updateExisting': True, 'assign': True}, status=404)
        imported = self.request('GET', '/members?q=' + phones[4])['items'][0]
        members.append(imported)
        # Round-trip an actual XLSX workbook through the import parser.
        template, _ = self.request('GET', '/imports/template?format=xlsx', raw=True)
        original = zipfile.ZipFile(io.BytesIO(template))
        sheet = ET.fromstring(original.read('xl/worksheets/sheet1.xml'))
        ns = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
        sheet_data = sheet.find(ns + 'sheetData')
        row = ET.SubElement(sheet_data, ns + 'row', {'r': '2'})
        for col, value in zip('ABC', ['Excel', suffix, phones[5]]):
            cell = ET.SubElement(row, ns + 'c', {'r': col + '2', 't': 'inlineStr'})
            ET.SubElement(ET.SubElement(cell, ns + 'is'), ns + 't').text = value
        output = io.BytesIO()
        with zipfile.ZipFile(output, 'w') as book:
            for name in original.namelist():
                book.writestr(name, ET.tostring(sheet, encoding='utf-8', xml_declaration=True) if name == 'xl/worksheets/sheet1.xml' else original.read(name))
        data = (f'--{boundary}\r\nContent-Disposition: form-data; name="eventId"\r\n\r\n{eid}\r\n--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="fixture.xlsx"\r\nContent-Type: application/octet-stream\r\n\r\n'.encode() + output.getvalue() + f'\r\n--{boundary}--\r\n'.encode())
        preview = self.request('POST', '/imports/preview', data, headers={'Content-Type': 'multipart/form-data; boundary=' + boundary})
        self.assertEqual(preview['summary']['new'], 1)
        self.request('POST', '/imports/' + preview['importId'] + '/commit', {'updateExisting': True, 'assign': False})
        members.append(self.request('GET', '/members?q=' + phones[5])['items'][0])
        # Two simultaneous admins cannot claim the same empty seat with replace=false.
        fresh = section['positions'][2]
        self.request('POST', f'/events/{eid}/assignments/bulk', {'memberIds': [m1['id'], m2['id']], 'zoneId': section['id'], 'positionId': fresh['id']}, status=422)
        def race(m):
            req = urllib.request.Request(BASE + f'/events/{eid}/assignments', data=json.dumps(assign(m, fresh, replace=False)).encode(), headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + self.token}, method='POST')
            try: return urllib.request.urlopen(req, timeout=30).status
            except urllib.error.HTTPError as e: return e.code
        with concurrent.futures.ThreadPoolExecutor(2) as pool:
            self.assertEqual(sorted(pool.map(race, [m1, m2])), [201, 409])
        # Session already started: refusal uses Nigeria's local time.
        past = self.request('PATCH', f'/events/{duplicate["id"]}', {'date': '2020-01-01'})
        past_a = self.request('GET', f'/events/{past["id"]}/assignments')['items'][0]
        self.request('PATCH', '/assignments/' + past_a['id'], {'status': 'unavailable'}, status=422)
        self.request('GET', f'/events/{eid}/overview')
        self.request('GET', '/team')
        self.request('GET', '/zones/names')
        self.request('GET', '/search?q=Section&eventId=' + eid)
        self.request('GET', '/activity')
        self.request('GET', '/notifications')
        self.request('POST', '/notifications/read-all', status=204)
        # Cascades leave no orphaned assignments.
        self.request('PUT', '/settings/current-event', {'eventId': duplicate['id']})
        self.request('DELETE', '/events/' + eid, status=204)
        self.request('GET', '/events/' + eid, status=404)
        self.request('GET', '/events/' + eid + '/assignments', status=404)
        self.request('PATCH', '/settings', {'currentEventId': None})
        self.request('DELETE', '/events/' + duplicate['id'], status=204)
        self.request('POST', '/members/bulk-delete', {'memberIds': [m['id'] for m in members]})
        # Password-reset delivery, single use, and session revocation.
        self.request('POST', '/auth/forgot-password', {'email': 'admin@example.test'}, token=False, status=204)
        reset = next(m['value'] for m in reversed(self.messages()) if m['kind'] == 'password-reset')
        self.request('POST', '/auth/reset-password', {'token': reset, 'password': 'local-test-password-123'}, token=False, status=204)
        self.request('POST', '/auth/reset-password', {'token': reset, 'password': 'local-test-password-123'}, token=False, status=401)
        self.request('GET', '/auth/me', status=401)
        new_login = self.request('POST', '/auth/login', {'email': 'admin@example.test', 'password': 'local-test-password-123'}, token=False)
        self.token = new_login['accessToken']
        self.request('POST', '/auth/logout', status=204)
        self.request('POST', '/auth/refresh', {'refreshToken': new_login['refreshToken']}, token=False, status=401)

if __name__ == '__main__':
    unittest.main()
