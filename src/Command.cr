require "woollib/common"
require "woollib/Command"

module Wool
  abstract struct Command(T)
    dc Users, add_user, {u: User}, begin
      s.add **@args
    end
  end
end
