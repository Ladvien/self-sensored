USE self_sensored;

-- Get the average heartrate
-- for a timeframe.
SELECT user_id, 
	   DATE_FORMAT(date, '%Y-%m-%d %H:%i'), 
 	   AVG(quantity)
FROM activities
GROUP BY user_id, DATE_FORMAT(date, '%Y-%m-%d %H:%i');

-- How many Activities are there for 
-- a particular user.
SELECT COUNT(a.id)
FROM activities AS a
WHERE a.user_id = 1;