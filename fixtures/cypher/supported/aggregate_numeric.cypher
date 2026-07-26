MATCH (n:Module) RETURN min(n.size) AS smallest, max(n.size) AS largest, avg(n.size) AS mean, sum(n.size) AS total
