module Wool
  class Users
    enum Site
      Telegram
      Max
    end

    class Integration
      mserializable

      getter user_name : User::Name
      getter site : Site
      getter pseudonym : String

      def_equals_and_hash @user_name, @site, @pseudonym

      getter id : Id { Id.from_serializable self }

      def initialize(@user_name, @site, @pseudonym)
      end
    end
  end
end
