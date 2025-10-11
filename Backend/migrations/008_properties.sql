CREATE TYPE property_status AS ENUM (
    'draft',
    'awaiting_agent',
    'agent_verified',
    'awaiting_lawyer',
    'lawyer_verified',
    'active',
    'suspended',
    'rejected',
    'sold',
    'rented'
);

CREATE TYPE property_type AS ENUM (
    'apartment',
    'house',
    'duplex',
    'bungalow',
    'commercial',
    'land',
    'warehouse',
    'office',
    'shop',
    'hotel'
);

CREATE TYPE listing_type AS ENUM (
    'sale',
    'rent',
    'lease',
    'asset'
);

CREATE TYPE currency_type AS ENUM (
    'naira',
    'usd',
    'vern'
);

CREATE TABLE properties (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    landlord_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    agent_id UUID REFERENCES users(id) ON DELETE SET NULL,
    lawyer_id UUID REFERENCES users(id) ON DELETE SET NULL,

    title VARCHAR(200) NOT NULL,
    description TEXT NOT NULL,
    property_type property_type NOT NULL,
    listing_type listing_type NOT NULL,

    address VARCHAR(500) NOT NULL,
    city VARCHAR(100) NOT NULL,
    state VARCHAR(100) NOT NULL,
    lga VARCHAR(100) NOT NULL,
    country VARCHAR(100) NOT NULL,
    latitude DECIMAL(10, 8),
    longitude DECIMAL(11, 8),
    landmark VARCHAR(200),

    bedrooms INTEGER,
    bathrooms INTEGER,
    toilets INTEGER,
    size_sqm DECIMAL(10, 2),
    plot_size VARCHAR(100),

    price BIGINT NOT NULL CHECK (price > 0),
    bidding_price BIGINT CHECK (bidding_price > price),
    currency currency_type NOT NULL DEFAULT 'naira',
    price_negotiable BOOLEAN DEFAULT false,

    amenities JSONB DEFAULT '[]'::jsonb,
    features JSONB DEFAULT '[]'::jsonb,

    certificate_of_occupancy VARCHAR(500),
    deed_of_agreement VARCHAR(500),
    survey_plan VARCHAR(500),
    building_plan_approval VARCHAR(500),

    property_photos JSONB NOT NULL DEFAULT '[]'::jsonb,
    agent_verification_photos JSONB,
    agent_verification_notes TEXT,
    lawyer_verification_notes TEXT,

    property_hash VARCHAR(64) NOT NULL,
    coordinates_hash VARCHAR(64) NOT NULL,

    status property_status NOT NULL DEFAULT 'awaiting_agent',
    agent_verified_at TIMESTAMP WITH TIME ZONE,
    lawyer_verified_at TIMESTAMP WITH TIME ZONE,
    listed_at TIMESTAMP WITH TIME ZONE,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_coordinates CHECK (
        (latitude IS NULL AND longitude IS NULL) OR
        (latitude IS NOT NULL AND longitude IS NOT NULL AND 
            latitude >= -90 AND latitude <= 90 AND 
            longitude >= -180 AND longitude <= 180)
    ),

    CONSTRAINT valid_rooms CHECK (
        bedrooms IS NULL OR bedrooms >= 0
    ),

    CONSTRAINT valid_bathrooms CHECK (
        bathrooms IS NULL OR bathrooms >= 0
    ),

    CONSTRAINT valid_toilets CHECK (
        toilets IS NULL OR toilets >= 0
    )
);

CREATE INDEX idx_properties_landlord_id ON properties(landlord_id);
CREATE INDEX idx_properties_agent_id ON properties(agent_id);
CREATE INDEX idx_properties_lawyer_id ON properties(lawyer_id);
CREATE INDEX idx_properties_status ON properties(status);
CREATE INDEX idx_properties_property_type ON properties(property_type);
CREATE INDEX idx_properties_listing_type ON properties(listing_type);
CREATE INDEX idx_properties_city ON properties(city);
CREATE INDEX idx_properties_state ON properties(state);
CREATE INDEX idx_properties_price ON properties(price);
CREATE INDEX idx_properties_bedrooms ON properties(bedrooms);
CREATE INDEX idx_properties_created_at ON properties(created_at);
CREATE INDEX idx_properties_listed_at ON properties(listed_at);

CREATE UNIQUE INDEX idx_properties_unique_hash ON properties(property_hash);
CREATE UNIQUE INDEX idx_properties_unique_coordinates ON properties(coordinates_hash)
    WHERE coordinates_hash != '0000000000000000000000000000000000000000000000000000000000000000';


CREATE TABLE property_verifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    property_id UUID NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
    verifier_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    verifier_type VARCHAR(20) NOT NULL CHECK (verifier_type IN ('agent', 'lawyer')),
    verification_status VARCHAR(20) NOT NULL CHECK (verification_status IN ('approved', 'rejected', 'pending')),
    notes TEXT NOT NULL,
    verification_photos JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_property_verifications_property_id ON property_verifications(property_id);
CREATE INDEX idx_property_verifications_verifier_id ON property_verifications(verifier_id);
CREATE INDEX idx_property_verifications_created_at ON property_verifications(created_at);

CREATE TABLE property_views (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    property_id UUID NOT NULL REFERENCES properties(id) On DELETE CASCADE,
    viewer_id UUID REFERENCES users(id) ON DELETE SET NULL,
    viewer_ip INET,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_property_views_property_id ON property_views(property_id);
CREATE INDEX idx_property_views_viewer_id ON property_views(viewer_id);
CREATE INDEX idx_property_views_created_at ON property_views(created_at);

CREATE TABLE property_favorites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    property_id UUID NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    UNIQUE(user_id, property_id)
);

CREATE INDEX idx_property_favorites_user_id ON property_favorites(user_id);
CREATE INDEX idx_property_favorites_property_id ON property_favorites(property_id);

CREATE TABLE property_inquiries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    property_id UUID NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
    inquirer_id UUID REFERENCES users(id) ON DELETE SET NULL,
    inquirer_name VARCHAR(255) NOT NULL,
    inquirer_email VARCHAR(255) NOT NULL,
    inquirer_phone VARCHAR(20),
    message TEXT NOT NULL,
    inquiry_type VARCHAR(20) DEFAULT 'general' CHECK (inquiry_type IN ('general', 'viewing', 'purchase', 'rent')),
    status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'responded', 'closed')),
    landlord_response TEXT,
    responded_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_property_inquiries_property_id ON property_inquiries(property_id);
CREATE INDEX idx_property_inquiries_inquirer_id ON property_inquiries(inquirer_id);
CREATE INDEX idx_property_inquiries_status ON property_inquiries(status);
CREATE INDEX idx_property_inquiries_created_at ON property_inquiries(created_at);