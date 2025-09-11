-- Create nutrition_metrics Table Migration
-- Creates comprehensive nutrition tracking table with 35+ Apple Health nutrition fields
-- Features: Complete macros, vitamins, minerals, hydration tracking with monthly partitioning

-- Create nutrition_metrics table with Apple Health field names
CREATE TABLE nutrition_metrics (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    
    -- === HYDRATION ===
    -- Water intake tracking
    water_ml NUMERIC(8,2),  -- Dietary Water (HKQuantityTypeIdentifierDietaryWater)
    
    -- === ENERGY & MACRONUTRIENTS ===
    -- Main energy and macronutrients with proper precision for nutrition tracking
    energy_consumed_kcal NUMERIC(8,2),  -- Calories consumed (HKQuantityTypeIdentifierDietaryEnergyConsumed)
    carbohydrates_g NUMERIC(8,2),       -- Total carbohydrates (HKQuantityTypeIdentifierDietaryCarbohydrates)
    protein_g NUMERIC(8,2),             -- Protein (HKQuantityTypeIdentifierDietaryProtein)
    fat_total_g NUMERIC(8,2),           -- Total fat (HKQuantityTypeIdentifierDietaryFatTotal)
    fat_saturated_g NUMERIC(8,2),       -- Saturated fat (HKQuantityTypeIdentifierDietaryFatSaturated)
    fat_monounsaturated_g NUMERIC(8,2), -- Monounsaturated fat (HKQuantityTypeIdentifierDietaryFatMonounsaturated)
    fat_polyunsaturated_g NUMERIC(8,2), -- Polyunsaturated fat (HKQuantityTypeIdentifierDietaryFatPolyunsaturated)
    cholesterol_mg NUMERIC(8,2),        -- Cholesterol (HKQuantityTypeIdentifierDietaryCholesterol)
    fiber_g NUMERIC(8,2),               -- Dietary fiber (HKQuantityTypeIdentifierDietaryFiber)
    sugar_g NUMERIC(8,2),               -- Sugar (HKQuantityTypeIdentifierDietarySugar)
    sodium_mg NUMERIC(8,2),             -- Sodium (HKQuantityTypeIdentifierDietarySodium)
    
    -- === VITAMINS ===
    -- Fat-soluble vitamins (A, D, E, K)
    vitamin_a_mcg NUMERIC(8,2),         -- Vitamin A (HKQuantityTypeIdentifierDietaryVitaminA)
    vitamin_d_mcg NUMERIC(8,2),         -- Vitamin D (HKQuantityTypeIdentifierDietaryVitaminD)
    vitamin_e_mg NUMERIC(8,2),          -- Vitamin E (HKQuantityTypeIdentifierDietaryVitaminE)
    vitamin_k_mcg NUMERIC(8,2),         -- Vitamin K (HKQuantityTypeIdentifierDietaryVitaminK)
    
    -- Water-soluble vitamins (B-complex and C)
    vitamin_c_mg NUMERIC(8,2),          -- Vitamin C (HKQuantityTypeIdentifierDietaryVitaminC)
    thiamin_mg NUMERIC(8,3),            -- Thiamin/B1 (HKQuantityTypeIdentifierDietaryThiamin)
    riboflavin_mg NUMERIC(8,3),         -- Riboflavin/B2 (HKQuantityTypeIdentifierDietaryRiboflavin)
    niacin_mg NUMERIC(8,2),             -- Niacin/B3 (HKQuantityTypeIdentifierDietaryNiacin)
    pantothenic_acid_mg NUMERIC(8,3),   -- Pantothenic acid/B5 (HKQuantityTypeIdentifierDietaryPantothenicAcid)
    vitamin_b6_mg NUMERIC(8,3),         -- Vitamin B6 (HKQuantityTypeIdentifierDietaryVitaminB6)
    biotin_mcg NUMERIC(8,2),            -- Biotin/B7 (HKQuantityTypeIdentifierDietaryBiotin)
    folate_mcg NUMERIC(8,2),            -- Folate/B9 (HKQuantityTypeIdentifierDietaryFolate)
    vitamin_b12_mcg NUMERIC(8,2),       -- Vitamin B12 (HKQuantityTypeIdentifierDietaryVitaminB12)
    
    -- === MINERALS ===
    -- Major minerals (>100mg daily requirement)
    calcium_mg NUMERIC(8,2),            -- Calcium (HKQuantityTypeIdentifierDietaryCalcium)
    phosphorus_mg NUMERIC(8,2),         -- Phosphorus (HKQuantityTypeIdentifierDietaryPhosphorus)
    magnesium_mg NUMERIC(8,2),          -- Magnesium (HKQuantityTypeIdentifierDietaryMagnesium)
    potassium_mg NUMERIC(8,2),          -- Potassium (HKQuantityTypeIdentifierDietaryPotassium)
    chloride_mg NUMERIC(8,2),           -- Chloride (HKQuantityTypeIdentifierDietaryChloride)
    
    -- Trace minerals (<100mg daily requirement)
    iron_mg NUMERIC(8,3),               -- Iron (HKQuantityTypeIdentifierDietaryIron)
    zinc_mg NUMERIC(8,3),               -- Zinc (HKQuantityTypeIdentifierDietaryZinc)
    copper_mg NUMERIC(8,3),             -- Copper (HKQuantityTypeIdentifierDietaryCopper)
    manganese_mg NUMERIC(8,3),          -- Manganese (HKQuantityTypeIdentifierDietaryManganese)
    iodine_mcg NUMERIC(8,2),            -- Iodine (HKQuantityTypeIdentifierDietaryIodine)
    selenium_mcg NUMERIC(8,2),          -- Selenium (HKQuantityTypeIdentifierDietarySelenium)
    chromium_mcg NUMERIC(8,2),          -- Chromium (HKQuantityTypeIdentifierDietaryChromium)
    molybdenum_mcg NUMERIC(8,2),        -- Molybdenum (HKQuantityTypeIdentifierDietaryMolybdenum)
    
    -- === OTHER NUTRIENTS ===
    caffeine_mg NUMERIC(8,2),           -- Caffeine (HKQuantityTypeIdentifierDietaryCaffeine)
    
    -- === METADATA ===
    source VARCHAR(100),                -- Data source (MyFitnessPal, Lose It, manual, etc.)
    raw_data JSONB,                     -- Store original payload for data recovery
    aggregation_period VARCHAR(20) DEFAULT 'daily' CHECK (aggregation_period IN ('meal', 'daily', 'weekly')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- === CONSTRAINTS ===
    PRIMARY KEY (user_id, recorded_at),
    UNIQUE (user_id, recorded_at),
    
    -- === VALIDATION CONSTRAINTS ===
    -- Hydration constraints (0-20L per day max, considering extreme cases)
    CONSTRAINT nutrition_water_check 
        CHECK (water_ml IS NULL OR (water_ml >= 0 AND water_ml <= 20000)),
    
    -- Energy constraints (0-20,000 kcal max for extreme cases)
    CONSTRAINT nutrition_energy_check 
        CHECK (energy_consumed_kcal IS NULL OR (energy_consumed_kcal >= 0 AND energy_consumed_kcal <= 20000)),
    
    -- Macronutrient constraints (reasonable daily maximums)
    CONSTRAINT nutrition_carbs_check 
        CHECK (carbohydrates_g IS NULL OR (carbohydrates_g >= 0 AND carbohydrates_g <= 3000)),
    CONSTRAINT nutrition_protein_check 
        CHECK (protein_g IS NULL OR (protein_g >= 0 AND protein_g <= 1000)),
    CONSTRAINT nutrition_fat_total_check 
        CHECK (fat_total_g IS NULL OR (fat_total_g >= 0 AND fat_total_g <= 1000)),
    CONSTRAINT nutrition_fat_saturated_check 
        CHECK (fat_saturated_g IS NULL OR (fat_saturated_g >= 0 AND fat_saturated_g <= 500)),
    CONSTRAINT nutrition_fat_monounsaturated_check 
        CHECK (fat_monounsaturated_g IS NULL OR (fat_monounsaturated_g >= 0 AND fat_monounsaturated_g <= 500)),
    CONSTRAINT nutrition_fat_polyunsaturated_check 
        CHECK (fat_polyunsaturated_g IS NULL OR (fat_polyunsaturated_g >= 0 AND fat_polyunsaturated_g <= 500)),
    CONSTRAINT nutrition_cholesterol_check 
        CHECK (cholesterol_mg IS NULL OR (cholesterol_mg >= 0 AND cholesterol_mg <= 5000)),
    CONSTRAINT nutrition_fiber_check 
        CHECK (fiber_g IS NULL OR (fiber_g >= 0 AND fiber_g <= 200)),
    CONSTRAINT nutrition_sugar_check 
        CHECK (sugar_g IS NULL OR (sugar_g >= 0 AND sugar_g <= 2000)),
    CONSTRAINT nutrition_sodium_check 
        CHECK (sodium_mg IS NULL OR (sodium_mg >= 0 AND sodium_mg <= 50000)),
    
    -- Vitamin constraints (based on ULs - Upper Levels from nutrition science)
    CONSTRAINT nutrition_vitamin_a_check 
        CHECK (vitamin_a_mcg IS NULL OR (vitamin_a_mcg >= 0 AND vitamin_a_mcg <= 10000)),
    CONSTRAINT nutrition_vitamin_c_check 
        CHECK (vitamin_c_mg IS NULL OR (vitamin_c_mg >= 0 AND vitamin_c_mg <= 5000)),
    CONSTRAINT nutrition_vitamin_d_check 
        CHECK (vitamin_d_mcg IS NULL OR (vitamin_d_mcg >= 0 AND vitamin_d_mcg <= 1000)),
    CONSTRAINT nutrition_vitamin_e_check 
        CHECK (vitamin_e_mg IS NULL OR (vitamin_e_mg >= 0 AND vitamin_e_mg <= 2000)),
    CONSTRAINT nutrition_vitamin_k_check 
        CHECK (vitamin_k_mcg IS NULL OR (vitamin_k_mcg >= 0 AND vitamin_k_mcg <= 5000)),
    CONSTRAINT nutrition_thiamin_check 
        CHECK (thiamin_mg IS NULL OR (thiamin_mg >= 0 AND thiamin_mg <= 100)),
    CONSTRAINT nutrition_riboflavin_check 
        CHECK (riboflavin_mg IS NULL OR (riboflavin_mg >= 0 AND riboflavin_mg <= 100)),
    CONSTRAINT nutrition_niacin_check 
        CHECK (niacin_mg IS NULL OR (niacin_mg >= 0 AND niacin_mg <= 1000)),
    CONSTRAINT nutrition_pantothenic_acid_check 
        CHECK (pantothenic_acid_mg IS NULL OR (pantothenic_acid_mg >= 0 AND pantothenic_acid_mg <= 100)),
    CONSTRAINT nutrition_vitamin_b6_check 
        CHECK (vitamin_b6_mg IS NULL OR (vitamin_b6_mg >= 0 AND vitamin_b6_mg <= 500)),
    CONSTRAINT nutrition_biotin_check 
        CHECK (biotin_mcg IS NULL OR (biotin_mcg >= 0 AND biotin_mcg <= 10000)),
    CONSTRAINT nutrition_folate_check 
        CHECK (folate_mcg IS NULL OR (folate_mcg >= 0 AND folate_mcg <= 5000)),
    CONSTRAINT nutrition_vitamin_b12_check 
        CHECK (vitamin_b12_mcg IS NULL OR (vitamin_b12_mcg >= 0 AND vitamin_b12_mcg <= 5000)),
    
    -- Mineral constraints (based on ULs and practical maximums)
    CONSTRAINT nutrition_calcium_check 
        CHECK (calcium_mg IS NULL OR (calcium_mg >= 0 AND calcium_mg <= 10000)),
    CONSTRAINT nutrition_phosphorus_check 
        CHECK (phosphorus_mg IS NULL OR (phosphorus_mg >= 0 AND phosphorus_mg <= 10000)),
    CONSTRAINT nutrition_magnesium_check 
        CHECK (magnesium_mg IS NULL OR (magnesium_mg >= 0 AND magnesium_mg <= 5000)),
    CONSTRAINT nutrition_potassium_check 
        CHECK (potassium_mg IS NULL OR (potassium_mg >= 0 AND potassium_mg <= 20000)),
    CONSTRAINT nutrition_chloride_check 
        CHECK (chloride_mg IS NULL OR (chloride_mg >= 0 AND chloride_mg <= 20000)),
    CONSTRAINT nutrition_iron_check 
        CHECK (iron_mg IS NULL OR (iron_mg >= 0 AND iron_mg <= 200)),
    CONSTRAINT nutrition_zinc_check 
        CHECK (zinc_mg IS NULL OR (zinc_mg >= 0 AND zinc_mg <= 200)),
    CONSTRAINT nutrition_copper_check 
        CHECK (copper_mg IS NULL OR (copper_mg >= 0 AND copper_mg <= 50)),
    CONSTRAINT nutrition_manganese_check 
        CHECK (manganese_mg IS NULL OR (manganese_mg >= 0 AND manganese_mg <= 50)),
    CONSTRAINT nutrition_iodine_check 
        CHECK (iodine_mcg IS NULL OR (iodine_mcg >= 0 AND iodine_mcg <= 5000)),
    CONSTRAINT nutrition_selenium_check 
        CHECK (selenium_mcg IS NULL OR (selenium_mcg >= 0 AND selenium_mcg <= 2000)),
    CONSTRAINT nutrition_chromium_check 
        CHECK (chromium_mcg IS NULL OR (chromium_mcg >= 0 AND chromium_mcg <= 2000)),
    CONSTRAINT nutrition_molybdenum_check 
        CHECK (molybdenum_mcg IS NULL OR (molybdenum_mcg >= 0 AND molybdenum_mcg <= 5000)),
    
    -- Other nutrient constraints
    CONSTRAINT nutrition_caffeine_check 
        CHECK (caffeine_mg IS NULL OR (caffeine_mg >= 0 AND caffeine_mg <= 2000))
    
) PARTITION BY RANGE (recorded_at);

-- === INDEXES FOR TIME-SERIES OPTIMIZATION ===
-- Create BRIN indexes for time-series optimization (most efficient for partitioned time-series data)
CREATE INDEX IF NOT EXISTS idx_nutrition_recorded_at_brin 
    ON nutrition_metrics USING BRIN (recorded_at);

CREATE INDEX IF NOT EXISTS idx_nutrition_user_recorded_brin 
    ON nutrition_metrics USING BRIN (user_id, recorded_at);

CREATE INDEX IF NOT EXISTS idx_nutrition_aggregation_period_brin 
    ON nutrition_metrics USING BRIN (aggregation_period, recorded_at);

-- Create B-tree index for frequently queried aggregation period
CREATE INDEX IF NOT EXISTS idx_nutrition_user_aggregation 
    ON nutrition_metrics (user_id, aggregation_period);

-- === MONTHLY PARTITIONING FUNCTIONS ===
-- Function to create monthly partitions specifically for nutrition_metrics
CREATE OR REPLACE FUNCTION create_nutrition_monthly_partitions(
    start_months_back integer DEFAULT 1,
    end_months_ahead integer DEFAULT 3
)
RETURNS void AS $$
DECLARE
    start_date date;
    end_date date;
    partition_name text;
    i integer;
BEGIN
    FOR i IN -start_months_back..end_months_ahead LOOP
        start_date := date_trunc('month', CURRENT_DATE) + (i || ' months')::interval;
        end_date := start_date + '1 month'::interval;
        partition_name := 'nutrition_metrics_' || to_char(start_date, 'YYYY_MM');
        
        -- Check if partition already exists
        IF NOT EXISTS (
            SELECT 1 FROM pg_class WHERE relname = partition_name
        ) THEN
            EXECUTE format('
                CREATE TABLE %I PARTITION OF nutrition_metrics
                FOR VALUES FROM (%L) TO (%L)',
                partition_name, start_date, end_date
            );
            
            -- Create indexes for this partition
            EXECUTE format('
                CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (recorded_at)',
                partition_name || '_recorded_at_brin', partition_name
            );
            
            EXECUTE format('
                CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (user_id, recorded_at)',
                partition_name || '_user_recorded_brin', partition_name
            );
            
            EXECUTE format('
                CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (aggregation_period, recorded_at)',
                partition_name || '_aggregation_recorded_brin', partition_name
            );
            
            RAISE NOTICE 'Created nutrition partition and indexes: %', partition_name;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Create initial partitions (3 months ahead as per requirements)
SELECT create_nutrition_monthly_partitions();

-- === UPDATE MAINTENANCE FUNCTION ===
-- Update the main partition maintenance function to include nutrition_metrics
CREATE OR REPLACE FUNCTION maintain_partitions()
RETURNS void AS $$
BEGIN
    -- Maintain partitions for all partitioned tables
    PERFORM create_monthly_partitions('raw_ingestions_partitioned', 'received_at');
    PERFORM create_monthly_partitions('audit_log_partitioned', 'created_at');
    PERFORM create_monthly_partitions('heart_rate_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('blood_pressure_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('activity_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('sleep_metrics_partitioned', 'date');
    
    -- Add new table maintenance
    PERFORM create_activity_v2_monthly_partitions();
    PERFORM create_nutrition_monthly_partitions();
END;
$$ LANGUAGE plpgsql;

-- === SUMMARY VIEWS ===
-- Create useful views for nutrition analysis
CREATE VIEW nutrition_metrics_daily_summary AS
SELECT 
    user_id,
    date_trunc('day', recorded_at) as nutrition_date,
    
    -- Hydration summary
    SUM(water_ml) as total_water_ml,
    
    -- Energy and macronutrient summaries
    SUM(energy_consumed_kcal) as total_energy_kcal,
    SUM(carbohydrates_g) as total_carbs_g,
    SUM(protein_g) as total_protein_g,
    SUM(fat_total_g) as total_fat_g,
    SUM(fiber_g) as total_fiber_g,
    SUM(sugar_g) as total_sugar_g,
    SUM(sodium_mg) as total_sodium_mg,
    
    -- Key vitamin summaries
    SUM(vitamin_c_mg) as total_vitamin_c_mg,
    SUM(vitamin_d_mcg) as total_vitamin_d_mcg,
    SUM(vitamin_b12_mcg) as total_vitamin_b12_mcg,
    
    -- Key mineral summaries
    SUM(calcium_mg) as total_calcium_mg,
    SUM(iron_mg) as total_iron_mg,
    SUM(magnesium_mg) as total_magnesium_mg,
    SUM(potassium_mg) as total_potassium_mg,
    
    -- Other
    SUM(caffeine_mg) as total_caffeine_mg,
    
    -- Metadata
    COUNT(*) as total_entries,
    array_agg(DISTINCT source) as data_sources,
    array_agg(DISTINCT aggregation_period) as aggregation_periods_used
FROM nutrition_metrics
WHERE aggregation_period IN ('meal', 'daily')
GROUP BY user_id, date_trunc('day', recorded_at);

-- === PERFORMANCE MONITORING ===
-- Performance monitoring function for nutrition_metrics
CREATE OR REPLACE FUNCTION analyze_nutrition_performance()
RETURNS TABLE (
    table_name text,
    partition_count bigint,
    total_rows bigint,
    avg_rows_per_partition bigint,
    oldest_data timestamptz,
    newest_data timestamptz,
    total_fields_with_data bigint
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        'nutrition_metrics'::text as table_name,
        COUNT(*)::bigint as partition_count,
        SUM(n_tup_ins)::bigint as total_rows,
        CASE WHEN COUNT(*) > 0 THEN (SUM(n_tup_ins) / COUNT(*))::bigint ELSE 0 END as avg_rows_per_partition,
        (SELECT MIN(recorded_at) FROM nutrition_metrics) as oldest_data,
        (SELECT MAX(recorded_at) FROM nutrition_metrics) as newest_data,
        37::bigint as total_fields_with_data  -- 35+ nutrition fields plus metadata
    FROM pg_stat_user_tables 
    WHERE relname LIKE 'nutrition_metrics_%';
END;
$$ LANGUAGE plpgsql;

-- === DOCUMENTATION ===
-- Add comprehensive documentation
COMMENT ON TABLE nutrition_metrics IS 'Comprehensive Apple Health nutrition tracking with 35+ fields including macronutrients, vitamins, minerals, and hydration. Features monthly partitioning and BRIN indexes for time-series optimization.';

-- Field documentation
COMMENT ON COLUMN nutrition_metrics.recorded_at IS 'Timestamp for nutrition entry with timezone support';
COMMENT ON COLUMN nutrition_metrics.water_ml IS 'Water intake in milliliters (Apple Health: HKQuantityTypeIdentifierDietaryWater)';
COMMENT ON COLUMN nutrition_metrics.energy_consumed_kcal IS 'Calories consumed (Apple Health: HKQuantityTypeIdentifierDietaryEnergyConsumed)';
COMMENT ON COLUMN nutrition_metrics.carbohydrates_g IS 'Total carbohydrates in grams (Apple Health: HKQuantityTypeIdentifierDietaryCarbohydrates)';
COMMENT ON COLUMN nutrition_metrics.protein_g IS 'Protein in grams (Apple Health: HKQuantityTypeIdentifierDietaryProtein)';
COMMENT ON COLUMN nutrition_metrics.fat_total_g IS 'Total fat in grams (Apple Health: HKQuantityTypeIdentifierDietaryFatTotal)';
COMMENT ON COLUMN nutrition_metrics.vitamin_c_mg IS 'Vitamin C in milligrams (Apple Health: HKQuantityTypeIdentifierDietaryVitaminC)';
COMMENT ON COLUMN nutrition_metrics.calcium_mg IS 'Calcium in milligrams (Apple Health: HKQuantityTypeIdentifierDietaryCalcium)';
COMMENT ON COLUMN nutrition_metrics.iron_mg IS 'Iron in milligrams (Apple Health: HKQuantityTypeIdentifierDietaryIron)';
COMMENT ON COLUMN nutrition_metrics.caffeine_mg IS 'Caffeine in milligrams (Apple Health: HKQuantityTypeIdentifierDietaryCaffeine)';
COMMENT ON COLUMN nutrition_metrics.aggregation_period IS 'Data granularity: meal-level, daily total, or weekly summary';
COMMENT ON COLUMN nutrition_metrics.raw_data IS 'Original payload stored for data recovery and debugging';