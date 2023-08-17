create table person
(
    person_id uuid primary key default gen_random_uuid()
);

SELECT trigger_updated_at('person');
