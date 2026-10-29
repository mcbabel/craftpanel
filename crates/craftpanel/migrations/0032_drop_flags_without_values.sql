UPDATE servers
   SET extra_flags = (
       SELECT json_group_array(flag.value)
         FROM json_each(servers.extra_flags) AS flag
        WHERE flag.value NOT IN ('--add-modules', '--add-opens', '--add-exports', '--add-reads',
                                 '--enable-native-access', '--module-path', '--upgrade-module-path',
                                 '--patch-module', '--limit-modules', '--class-path')
   )
 WHERE EXISTS (
       SELECT 1
         FROM json_each(servers.extra_flags) AS flag
        WHERE flag.value IN ('--add-modules', '--add-opens', '--add-exports', '--add-reads',
                             '--enable-native-access', '--module-path', '--upgrade-module-path',
                             '--patch-module', '--limit-modules', '--class-path')
   );
