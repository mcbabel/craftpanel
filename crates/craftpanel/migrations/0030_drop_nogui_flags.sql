UPDATE servers
   SET extra_flags = (
       SELECT json_group_array(flag.value)
         FROM json_each(servers.extra_flags) AS flag
        WHERE flag.value NOT IN ('nogui', '-nogui', '--nogui')
   )
 WHERE EXISTS (
       SELECT 1
         FROM json_each(servers.extra_flags) AS flag
        WHERE flag.value IN ('nogui', '-nogui', '--nogui')
   );
