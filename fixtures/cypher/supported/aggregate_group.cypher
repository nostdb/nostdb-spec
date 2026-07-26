MATCH (n) RETURN n.language AS language, count(n) AS total ORDER BY language
