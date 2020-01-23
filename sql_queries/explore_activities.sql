USE self_sensored;

-- Get the average heartrate
-- for a timeframe.
SELECT user_id, 
	   DATE_FORMAT(date, '%Y-%m-%d %H:%i') AS date, 
 	   AVG(quantity) 					AS bpm,
 	   COUNT(id) 						AS number_of_samles
FROM activities
GROUP BY user_id, DATE_FORMAT(date, '%Y-%m-%d %H:%i')
ORDER BY date DESC;

-- How many Activities are there for 
-- a particular user.
SELECT COUNT(a.id)
FROM activities AS a
WHERE a.user_id = 1;

SELECT * FROM activities ORDER BY id DESC;

SELECT MAX(date) FROM activities WHERE user_id = '1' AND activity_type = 'HKQuantityTypeIdentifierRestingHeartRate';

SELECT * FROM activities WHERE activity_type LIKE '%BodyMass' ORDER BY date ASC;