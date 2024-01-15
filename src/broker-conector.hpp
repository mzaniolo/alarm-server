#pragma once

#include <string_view>

class BrokerConector
{
    public:
    bool connect(std::string_view host);
    bool subscribe(std::string_view meas);
    bool unsubscribe(std::string_view meas)

};
