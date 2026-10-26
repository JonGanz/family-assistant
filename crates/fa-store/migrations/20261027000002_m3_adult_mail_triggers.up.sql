-- D-061: adults get the new M3 triggers on existing notify prefs (new prefs get them from fa-notify).
SELECT set_config('app.system', 'on', true);
UPDATE notify_pref p
   SET triggers = p.triggers || ARRAY(SELECT t FROM unnest(ARRAY['approval_needed', 'mail_time_sensitive']) t
                                      WHERE NOT t = ANY (p.triggers))
  FROM member m
 WHERE m.id = p.member_id AND m.role = 'adult';
