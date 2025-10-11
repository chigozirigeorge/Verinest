-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for updating updated_at
CREATE OR REPLACE TRIGGER update_properties_updated_at
    BEFORE UPDATE ON properties
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- Function to automatically set listed_at based on status changes
CREATE OR REPLACE FUNCTION set_property_listed_at()
RETURNS TRIGGER AS $$
BEGIN
    -- If status is changing to active and listed_at is not set
    IF NEW.status = 'active' AND OLD.status != 'active' AND NEW.listed_at IS NULL THEN
        NEW.listed_at = NOW();
    END IF;

    -- If status is changing from active to something else, clear listed_at
    IF NEW.status != 'active' AND OLD.status = 'active' THEN
        NEW.listed_at = NULL;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for setting listed_at
CREATE OR REPLACE TRIGGER set_property_listed_at_trigger
    BEFORE UPDATE ON properties
    FOR EACH ROW
    EXECUTE FUNCTION set_property_listed_at();

-- Create a view for property statistics
CREATE OR REPLACE VIEW property_stats AS 
SELECT
    p.id,
    p.title,
    p.status,
    p.created_at,
    p.listed_at,
    COALESCE(views.view_count, 0) as view_count,
    COALESCE(favorites.favorite_count, 0) as favorite_count,
    COALESCE(inquiries.inquiry_count, 0) as inquiry_count,
    COALESCE(inquiries.pending_inquiries, 0) as pending_inquiries
FROM properties p
LEFT JOIN (
    SELECT
        property_id,
        COUNT(*) as view_count
    FROM property_views
    GROUP BY property_id
) views ON p.id = views.property_id
LEFT JOIN (
    SELECT
        property_id,
        COUNT(*) as favorite_count
    FROM property_favorites
    GROUP BY property_id
) favorites ON p.id = favorites.property_id
LEFT JOIN (
    SELECT 
        property_id,
        COUNT(*) as inquiry_count,
        COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_inquiries
    FROM property_inquiries
    GROUP BY property_id
) inquiries ON p.id = inquiries.property_id;