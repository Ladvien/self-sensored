# Apple HealthKit & Health Auto Export - Complete Intersection Table

## Data Export Formats by Type

| Data Type | CSV | JSON | GPX | Notes |
|-----------|-----|------|-----|-------|
| Quantity Types | ✅ | ✅ | ❌ | Numerical data with aggregation |
| Category Types | ✅ | ✅ | ❌ | Categorical/event data |
| Workouts | ✅ | ✅ | ✅ | GPX for route data only |
| Sleep Phases | Limited | ✅ | ❌ | Full phases in JSON only |
| ECG Data | ❌ | ✅ | ❌ | JSON format required |
| Symptoms | ✅ | ✅ | ❌ | Bulk export supported |
| Clinical Records | ❌ | Limited | ❌ | Very limited support |


## Data Types Support Matrix

| Category | HealthKit Identifier | Description | Health Auto Export Support | Notes |
|----------|---------------------|-------------|-------------|--------|
| **ACTIVITY & FITNESS** | | | | |
| Activity | `HKQuantityTypeIdentifierStepCount` | Step count | ✅ | Full support with aggregation |
| Activity | `HKQuantityTypeIdentifierDistanceWalkingRunning` | Walking + Running distance | ✅ | Combined metric |
| Activity | `HKQuantityTypeIdentifierDistanceCycling` | Cycling distance | ✅ | |
| Activity | `HKQuantityTypeIdentifierDistanceSwimming` | Swimming distance | ✅ | |
| Activity | `HKQuantityTypeIdentifierDistanceWheelchair` | Wheelchair distance | ✅ | |
| Activity | `HKQuantityTypeIdentifierDistanceDownhillSnowSports` | Skiing/Snowboarding distance | ✅ | |
| Activity | `HKQuantityTypeIdentifierFlightsClimbed` | Flights of stairs | ✅ | |
| Activity | `HKQuantityTypeIdentifierPushCount` | Wheelchair pushes | ✅ | |
| Activity | `HKQuantityTypeIdentifierSwimmingStrokeCount` | Swimming strokes | ✅ | |
| Activity | `HKQuantityTypeIdentifierNikeFuel` | NikeFuel points | ⚠️ | Uncertain |
| Apple Fitness | `HKQuantityTypeIdentifierAppleExerciseTime` | Exercise minutes | ✅ | |
| Apple Fitness | `HKQuantityTypeIdentifierAppleStandTime` | Stand time | ✅ | |
| Apple Fitness | `HKQuantityTypeIdentifierAppleMoveTime` | Move time | ✅ | |
| Apple Fitness | `HKCategoryTypeIdentifierAppleStandHour` | Stand hour achieved | ✅ | |
| **ENERGY** | | | | |
| Energy | `HKQuantityTypeIdentifierActiveEnergyBurned` | Active calories | ✅ | |
| Energy | `HKQuantityTypeIdentifierBasalEnergyBurned` | Resting calories | ✅ | |
| **HEART & CARDIOVASCULAR** | | | | |
| Heart | `HKQuantityTypeIdentifierHeartRate` | Heart rate | ✅ | All heart rate data |
| Heart | `HKQuantityTypeIdentifierRestingHeartRate` | Resting heart rate | ✅ | |
| Heart | `HKQuantityTypeIdentifierWalkingHeartRateAverage` | Walking heart rate avg | ✅ | |
| Heart | `HKQuantityTypeIdentifierHeartRateVariabilitySDNN` | Heart rate variability | ✅ | |
| Heart | `HKQuantityTypeIdentifierHeartRateRecoveryOneMinute` | Heart rate recovery | ✅ | |
| Heart | `HKQuantityTypeIdentifierAtrialFibrillationBurden` | AFib burden % | ✅ | |
| Blood Pressure | `HKQuantityTypeIdentifierBloodPressureSystolic` | Systolic BP | ✅ | |
| Blood Pressure | `HKQuantityTypeIdentifierBloodPressureDiastolic` | Diastolic BP | ✅ | |
| Blood Pressure | `HKCorrelationTypeIdentifierBloodPressure` | Blood pressure correlation | ✅ | Combined reading |
| Heart Events | `HKCategoryTypeIdentifierHighHeartRateEvent` | High heart rate event | ✅ | |
| Heart Events | `HKCategoryTypeIdentifierLowHeartRateEvent` | Low heart rate event | ✅ | |
| Heart Events | `HKCategoryTypeIdentifierIrregularHeartRhythmEvent` | Irregular rhythm | ✅ | |
| ECG | `HKElectrocardiogramType` | ECG recording | ✅ | Export as JSON |
| Cardio Fitness | `HKQuantityTypeIdentifierVO2Max` | VO2 Max | ✅ | |
| Cardio Fitness | `HKCategoryTypeIdentifierLowCardioFitnessEvent` | Low cardio fitness | ✅ | |
| **RESPIRATORY** | | | | |
| Respiratory | `HKQuantityTypeIdentifierRespiratoryRate` | Respiratory rate | ✅ | |
| Respiratory | `HKQuantityTypeIdentifierOxygenSaturation` | Blood oxygen (SpO2) | ✅ | |
| Respiratory | `HKQuantityTypeIdentifierForcedVitalCapacity` | Forced vital capacity | ✅ | |
| Respiratory | `HKQuantityTypeIdentifierForcedExpiratoryVolume1` | FEV1 | ✅ | |
| Respiratory | `HKQuantityTypeIdentifierPeakExpiratoryFlowRate` | Peak flow rate | ✅ | |
| Respiratory | `HKQuantityTypeIdentifierInhalerUsage` | Inhaler usage | ✅ | |
| **BODY MEASUREMENTS** | | | | |
| Body | `HKQuantityTypeIdentifierBodyMass` | Body weight | ✅ | |
| Body | `HKQuantityTypeIdentifierBodyMassIndex` | BMI | ✅ | |
| Body | `HKQuantityTypeIdentifierBodyFatPercentage` | Body fat % | ✅ | |
| Body | `HKQuantityTypeIdentifierLeanBodyMass` | Lean body mass | ✅ | |
| Body | `HKQuantityTypeIdentifierHeight` | Height | ✅ | |
| Body | `HKQuantityTypeIdentifierWaistCircumference` | Waist circumference | ⚠️ | Likely supported |
| Temperature | `HKQuantityTypeIdentifierBodyTemperature` | Body temperature | ✅ | |
| Temperature | `HKQuantityTypeIdentifierBasalBodyTemperature` | Basal body temp | ✅ | |
| Temperature | `HKQuantityTypeIdentifierAppleSleepingWristTemperature` | Wrist temperature | ✅ | |
| Temperature | `HKQuantityTypeIdentifierWaterTemperature` | Water temperature | ✅ | |
| **SLEEP** | | | | |
| Sleep | `HKCategoryTypeIdentifierSleepAnalysis` | Sleep stages | ✅ | Asleep/In Bed/Phases in JSON |
| Sleep | `HKQuantityTypeIdentifierSleepDurationGoal` | Sleep goal | ⚠️ | Uncertain |
| **NUTRITION** | | | | |
| Hydration | `HKQuantityTypeIdentifierDietaryWater` | Water intake | ✅ | |
| Macros | `HKQuantityTypeIdentifierDietaryEnergyConsumed` | Calories consumed | ✅ | |
| Macros | `HKQuantityTypeIdentifierDietaryCarbohydrates` | Carbohydrates | ✅ | |
| Macros | `HKQuantityTypeIdentifierDietaryProtein` | Protein | ✅ | |
| Macros | `HKQuantityTypeIdentifierDietaryFatTotal` | Total fat | ✅ | |
| Macros | `HKQuantityTypeIdentifierDietaryFatSaturated` | Saturated fat | ✅ | |
| Macros | `HKQuantityTypeIdentifierDietaryFatMonounsaturated` | Monounsaturated fat | ⚠️ | Likely supported |
| Macros | `HKQuantityTypeIdentifierDietaryFatPolyunsaturated` | Polyunsaturated fat | ⚠️ | Likely supported |
| Macros | `HKQuantityTypeIdentifierDietaryCholesterol` | Cholesterol | ✅ | |
| Macros | `HKQuantityTypeIdentifierDietarySodium` | Sodium | ✅ | |
| Macros | `HKQuantityTypeIdentifierDietaryFiber` | Fiber | ✅ | |
| Macros | `HKQuantityTypeIdentifierDietarySugar` | Sugar | ✅ | |
| Vitamins | `HKQuantityTypeIdentifierDietaryVitaminA` | Vitamin A | ✅ | |
| Vitamins | `HKQuantityTypeIdentifierDietaryVitaminB6` | Vitamin B6 | ⚠️ | Likely supported |
| Vitamins | `HKQuantityTypeIdentifierDietaryVitaminB12` | Vitamin B12 | ⚠️ | Likely supported |
| Vitamins | `HKQuantityTypeIdentifierDietaryVitaminC` | Vitamin C | ✅ | |
| Vitamins | `HKQuantityTypeIdentifierDietaryVitaminD` | Vitamin D | ✅ | |
| Vitamins | `HKQuantityTypeIdentifierDietaryVitaminE` | Vitamin E | ⚠️ | Likely supported |
| Vitamins | `HKQuantityTypeIdentifierDietaryVitaminK` | Vitamin K | ⚠️ | Likely supported |
| Vitamins | `HKQuantityTypeIdentifierDietaryThiamin` | Thiamin (B1) | ⚠️ | Uncertain |
| Vitamins | `HKQuantityTypeIdentifierDietaryRiboflavin` | Riboflavin (B2) | ⚠️ | Uncertain |
| Vitamins | `HKQuantityTypeIdentifierDietaryNiacin` | Niacin (B3) | ⚠️ | Uncertain |
| Vitamins | `HKQuantityTypeIdentifierDietaryFolate` | Folate | ⚠️ | Uncertain |
| Vitamins | `HKQuantityTypeIdentifierDietaryBiotin` | Biotin | ⚠️ | Uncertain |
| Vitamins | `HKQuantityTypeIdentifierDietaryPantothenicAcid` | Pantothenic acid | ⚠️ | Uncertain |
| Minerals | `HKQuantityTypeIdentifierDietaryCalcium` | Calcium | ✅ | |
| Minerals | `HKQuantityTypeIdentifierDietaryIron` | Iron | ✅ | |
| Minerals | `HKQuantityTypeIdentifierDietaryMagnesium` | Magnesium | ✅ | |
| Minerals | `HKQuantityTypeIdentifierDietaryPhosphorus` | Phosphorus | ⚠️ | Uncertain |
| Minerals | `HKQuantityTypeIdentifierDietaryPotassium` | Potassium | ✅ | |
| Minerals | `HKQuantityTypeIdentifierDietaryZinc` | Zinc | ⚠️ | Uncertain |
| Minerals | `HKQuantityTypeIdentifierDietaryIodine` | Iodine | ⚠️ | Uncertain |
| Minerals | `HKQuantityTypeIdentifierDietarySelenium` | Selenium | ⚠️ | Uncertain |
| Minerals | `HKQuantityTypeIdentifierDietaryCopper` | Copper | ⚠️ | Uncertain |
| Minerals | `HKQuantityTypeIdentifierDietaryManganese` | Manganese | ⚠️ | Uncertain |
| Minerals | `HKQuantityTypeIdentifierDietaryChromium` | Chromium | ⚠️ | Uncertain |
| Minerals | `HKQuantityTypeIdentifierDietaryMolybdenum` | Molybdenum | ⚠️ | Uncertain |
| Minerals | `HKQuantityTypeIdentifierDietaryChloride` | Chloride | ⚠️ | Uncertain |
| Other | `HKQuantityTypeIdentifierDietaryCaffeine` | Caffeine | ✅ | |
| **BLOOD & METABOLIC** | | | | |
| Blood | `HKQuantityTypeIdentifierBloodGlucose` | Blood glucose | ✅ | With metadata |
| Blood | `HKQuantityTypeIdentifierBloodAlcoholContent` | Blood alcohol | ✅ | |
| Blood | `HKQuantityTypeIdentifierInsulinDelivery` | Insulin delivery | ✅ | |
| Blood | `HKQuantityTypeIdentifierPeripheralPerfusionIndex` | Perfusion index | ⚠️ | Uncertain |
| **MINDFULNESS & MENTAL** | | | | |
| Mindfulness | `HKCategoryTypeIdentifierMindfulSession` | Mindful sessions | ✅ | |
| Mental Health | `HKStateOfMind` | State of mind | ✅ | iOS 17+ |
| **REPRODUCTIVE HEALTH** | | | | |
| Menstrual | `HKCategoryTypeIdentifierMenstrualFlow` | Menstrual flow | ✅ | |
| Menstrual | `HKCategoryTypeIdentifierIntermenstrualBleeding` | Spotting | ⚠️ | Likely supported |
| Menstrual | `HKCategoryTypeIdentifierInfrequentMenstrualCycles` | Infrequent cycles | ⚠️ | iOS 17+ |
| Menstrual | `HKCategoryTypeIdentifierIrregularMenstrualCycles` | Irregular cycles | ⚠️ | iOS 17+ |
| Menstrual | `HKCategoryTypeIdentifierPersistentIntermenstrualBleeding` | Persistent bleeding | ⚠️ | iOS 17+ |
| Menstrual | `HKCategoryTypeIdentifierProlongedMenstrualPeriods` | Prolonged periods | ⚠️ | iOS 17+ |
| Fertility | `HKCategoryTypeIdentifierCervicalMucusQuality` | Cervical mucus | ✅ | |
| Fertility | `HKCategoryTypeIdentifierOvulationTestResult` | Ovulation test | ✅ | |
| Fertility | `HKCategoryTypeIdentifierSexualActivity` | Sexual activity | ✅ | |
| Fertility | `HKCategoryTypeIdentifierPregnancyTestResult` | Pregnancy test | ✅ | |
| Fertility | `HKCategoryTypeIdentifierProgesteroneTestResult` | Progesterone test | ⚠️ | Uncertain |
| Fertility | `HKCategoryTypeIdentifierContraceptive` | Contraceptive use | ⚠️ | Uncertain |
| Fertility | `HKCategoryTypeIdentifierLactation` | Lactation | ⚠️ | Uncertain |
| Fertility | `HKCategoryTypeIdentifierPregnancy` | Pregnancy status | ⚠️ | Uncertain |
| **SYMPTOMS** | | | | |
| Symptoms | `HKCategoryTypeIdentifierAbdominalCramps` | Abdominal cramps | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierBloating` | Bloating | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierBreastPain` | Breast pain | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierHeadache` | Headache | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierFatigue` | Fatigue | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierNausea` | Nausea | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierDizziness` | Dizziness | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierFever` | Fever | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierCoughing` | Coughing | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierShortnessOfBreath` | Shortness of breath | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierChestTightnessOrPain` | Chest pain | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierPelvicPain` | Pelvic pain | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierRapidPoundingOrFlutteringHeartbeat` | Palpitations | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierSkippedHeartbeat` | Skipped heartbeat | ✅ | Export supported |
| Symptoms | `HKCategoryTypeIdentifierAcne` | Acne | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierAppetiteChanges` | Appetite changes | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierBladderIncontinence` | Bladder incontinence | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierChills` | Chills | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierConstipation` | Constipation | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierDiarrhea` | Diarrhea | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierDrySkin` | Dry skin | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierFainting` | Fainting | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierGeneralizedBodyAche` | Body ache | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierHairLoss` | Hair loss | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierHeartburn` | Heartburn | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierHotFlashes` | Hot flashes | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierLossOfSmell` | Loss of smell | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierLossOfTaste` | Loss of taste | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierLowerBackPain` | Lower back pain | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierMemoryLapse` | Memory lapse | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierMoodChanges` | Mood changes | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierNightSweats` | Night sweats | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierRunnyNose` | Runny nose | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierSinusCongestion` | Sinus congestion | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierSleepChanges` | Sleep changes | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierSoreThroat` | Sore throat | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierVaginalDryness` | Vaginal dryness | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierVomiting` | Vomiting | ⚠️ | Likely in export |
| Symptoms | `HKCategoryTypeIdentifierWheezing` | Wheezing | ⚠️ | Likely in export |
| **ENVIRONMENTAL & SAFETY** | | | | |
| Audio | `HKQuantityTypeIdentifierEnvironmentalAudioExposure` | Environmental noise | ✅ | |
| Audio | `HKQuantityTypeIdentifierHeadphoneAudioExposure` | Headphone audio | ✅ | |
| Audio | `HKCategoryTypeIdentifierEnvironmentalAudioExposureEvent` | Audio exposure event | ✅ | |
| Audio | `HKCategoryTypeIdentifierHeadphoneAudioExposureEvent` | Headphone event | ✅ | |
| Audio | `HKQuantityTypeIdentifierEnvironmentalSoundReduction` | Sound reduction | ⚠️ | Uncertain |
| Environmental | `HKQuantityTypeIdentifierUVExposure` | UV exposure | ✅ | |
| Environmental | `HKQuantityTypeIdentifierTimeInDaylight` | Time in daylight | ✅ | |
| Safety | `HKQuantityTypeIdentifierNumberOfTimesFallen` | Fall count | ✅ | |
| Hygiene | `HKCategoryTypeIdentifierHandwashingEvent` | Handwashing | ✅ | |
| Hygiene | `HKCategoryTypeIdentifierToothbrushingEvent` | Toothbrushing | ✅ | |
| **MOBILITY METRICS** | | | | |
| Walking | `HKQuantityTypeIdentifierWalkingSpeed` | Walking speed | ⚠️ | iOS 14+ uncertain |
| Walking | `HKQuantityTypeIdentifierWalkingStepLength` | Step length | ⚠️ | iOS 14+ uncertain |
| Walking | `HKQuantityTypeIdentifierWalkingAsymmetryPercentage` | Walking asymmetry | ⚠️ | iOS 14+ uncertain |
| Walking | `HKQuantityTypeIdentifierWalkingDoubleSupportPercentage` | Double support | ⚠️ | iOS 14+ uncertain |
| Walking | `HKQuantityTypeIdentifierSixMinuteWalkTestDistance` | 6-min walk test | ⚠️ | Likely supported |
| Walking | `HKCategoryTypeIdentifierAppleWalkingSteadinessEvent` | Walking steadiness | ⚠️ | Uncertain |
| Stairs | `HKQuantityTypeIdentifierStairAscentSpeed` | Stair ascent speed | ⚠️ | iOS 14+ uncertain |
| Stairs | `HKQuantityTypeIdentifierStairDescentSpeed` | Stair descent speed | ⚠️ | iOS 14+ uncertain |
| Running | `HKQuantityTypeIdentifierGroundContactTime` | Ground contact time | ⚠️ | Uncertain |
| Running | `HKQuantityTypeIdentifierVerticalOscillation` | Vertical oscillation | ⚠️ | Uncertain |
| Running | `HKQuantityTypeIdentifierRunningStrideLength` | Running stride length | ⚠️ | Uncertain |
| Running | `HKQuantityTypeIdentifierRunningPower` | Running power | ⚠️ | Uncertain |
| Running | `HKQuantityTypeIdentifierRunningSpeed` | Running speed | ⚠️ | Uncertain |
| **CYCLING METRICS** | | | | |
| Cycling | `HKQuantityTypeIdentifierCyclingSpeed` | Cycling speed | ⚠️ | iOS 17+ uncertain |
| Cycling | `HKQuantityTypeIdentifierCyclingPower` | Cycling power | ⚠️ | iOS 17+ uncertain |
| Cycling | `HKQuantityTypeIdentifierCyclingCadence` | Cycling cadence | ⚠️ | iOS 17+ uncertain |
| Cycling | `HKQuantityTypeIdentifierCyclingFunctionalThresholdPower` | Cycling FTP | ⚠️ | iOS 17+ uncertain |
| **UNDERWATER** | | | | |
| Underwater | `HKQuantityTypeIdentifierUnderwaterDepth` | Underwater depth | ⚠️ | iOS 16+ uncertain |
| **CHARACTERISTICS** | | | | |
| Characteristics | `HKCharacteristicTypeIdentifierBiologicalSex` | Biological sex | ✅ | Static data |
| Characteristics | `HKCharacteristicTypeIdentifierBloodType` | Blood type | ✅ | Static data |
| Characteristics | `HKCharacteristicTypeIdentifierDateOfBirth` | Date of birth | ✅ | Static data |
| Characteristics | `HKCharacteristicTypeIdentifierFitzpatrickSkinType` | Skin type | ⚠️ | Likely supported |
| Characteristics | `HKCharacteristicTypeIdentifierWheelchairUse` | Wheelchair use | ⚠️ | Likely supported |
| Characteristics | `HKCharacteristicTypeIdentifierActivityMoveMode` | Move mode | ⚠️ | iOS 14+ uncertain |
| **CLINICAL RECORDS** | | | | |
| Clinical | `HKClinicalTypeIdentifierAllergyRecord` | Allergy records | ❌ | Limited support |
| Clinical | `HKClinicalTypeIdentifierConditionRecord` | Conditions | ❌ | Limited support |
| Clinical | `HKClinicalTypeIdentifierCoverageRecord` | Insurance coverage | ❌ | Limited support |
| Clinical | `HKClinicalTypeIdentifierImmunizationRecord` | Immunizations | ❌ | Limited support |
| Clinical | `HKClinicalTypeIdentifierLabResultRecord` | Lab results | ❌ | Limited support |
| Clinical | `HKClinicalTypeIdentifierMedicationRecord` | Medications | ❌ | Limited support |
| Clinical | `HKClinicalTypeIdentifierProcedureRecord` | Procedures | ❌ | Limited support |
| Clinical | `HKClinicalTypeIdentifierVitalSignRecord` | Vital signs | ❌ | Limited support |
| Clinical | `HKClinicalTypeIdentifierClinicalNoteRecord` | Clinical notes | ❌ | iOS 16+ |
| Clinical | `HKDocumentTypeIdentifierCDA` | CDA documents | ❌ | Not supported |
| Clinical | `HKFHIRResource` | FHIR resources | ❌ | Not supported |
| **SPECIALIZED** | | | | |
| Specialized | `HKAudiogramSampleType` | Audiogram | ❌ | Not supported |
| Specialized | `HKVisionPrescriptionType` | Vision prescription | ❌ | iOS 16+ |
| Specialized | `HKQuantityTypeIdentifierElectrodermalActivity` | Electrodermal activity | ⚠️ | Uncertain |
| Specialized | `HKQuantityTypeIdentifierPhysicalEffort` | Physical effort | ⚠️ | Uncertain |
| **WORKOUTS** | | | | |
| Workouts | `HKWorkoutType` | All workout types | ✅ | 70+ types supported |
| Workouts | `HKWorkoutRouteType` | Workout GPS routes | ✅ | GPX export |
| Workouts | `HKActivitySummaryType` | Activity rings | ✅ | Daily summaries |
| **DATA SERIES** | | | | |
| Series | `HKDataTypeIdentifierHeartbeatSeries` | Beat-to-beat data | ⚠️ | Uncertain |
| Series | `HKSeriesType` | High-frequency data | ❌ | Not supported |

## Legend

- ✅ **Supported**: Confirmed support in Health Auto Export
- ⚠️ **Partial/Uncertain**: May be supported, unclear documentation, or limited functionality
- ❌ **Not Supported**: Not available in Health Auto Export

## Summary Statistics

| Support Level | Count | Percentage |
|---------------|-------|------------|
| ✅ Fully Supported | ~90 | ~45% |
| ⚠️ Partial/Uncertain | ~85 | ~42% |
| ❌ Not Supported | ~25 | ~13% |
| **Total HealthKit Types** | **~200** | **100%** |

## Key Observations

### Strong Support Areas (>80% coverage):
- Core fitness and activity metrics
- Heart and cardiovascular data
- Basic body measurements
- Energy and calorie tracking
- Sleep analysis (with some limitations)
- Environmental monitoring
- Reproductive health basics
- Workout tracking

### Moderate Support Areas (40-80% coverage):
- Detailed nutrition (vitamins/minerals)
- Advanced symptoms tracking
- Mobility metrics (newer iOS features)
- Cycling-specific metrics
- Some reproductive health details

### Weak Support Areas (<40% coverage):
- Clinical health records
- Medical documents (CDA, FHIR)
- Specialized medical devices (audiograms, vision)
- High-frequency sensor data series
- Very newest iOS 18 features

## Notes on Support Levels

1. **Version Dependencies**: Many uncertain items depend on iOS/watchOS version compatibility
2. **Export vs Display**: Some items may be exportable but not individually selectable
3. **Aggregation**: Supported items typically allow time-based aggregation (seconds to years)
4. **Metadata**: JSON format often includes additional metadata not available in CSV
5. **Updates**: Health Auto Export regularly updates to support new HealthKit features


## Recommendations by Use Case

### ✅ Excellent for:
- Personal health tracking
- Fitness and workout analysis
- Research and quantified self
- Home automation integration
- Health dashboards
- Routine vital sign monitoring

### ⚠️ Consider alternatives for:
- Complete clinical record management
- Medical device integration (beyond basics)
- Raw sensor data analysis
- FHIR/HL7 compliance needs
- Very latest iOS features (wait for updates)

### ❌ Not suitable for:
- Medical record interchange
- Clinical decision support
- Regulatory compliance requiring all data
- High-frequency physiological research