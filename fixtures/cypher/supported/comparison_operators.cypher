MATCH (n:Function) WHERE n.count > 3 AND n.ratio <= 0.5 OR NOT n.exported RETURN n
